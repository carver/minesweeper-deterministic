#!/bin/sh
# Builds the game, installs it with cargo, and adds a launcher entry and icon
# for the current user.
set -eu

usage() {
    cat <<USAGE
Usage: $0 [-h] [--uninstall]

Installs minesweeper-deterministic for the current user:
  binary    via cargo install (into \$CARGO_HOME/bin, usually ~/.cargo/bin)
  launcher  ~/.local/share/applications/minesweeper-deterministic.desktop
  icon      ~/.local/share/icons/hicolor/scalable/apps/minesweeper-deterministic.svg

  --uninstall  remove all three
  -h, --help   show this help
USAGE
}

root=$(cd "$(dirname "$0")/.." && pwd)
data=${XDG_DATA_HOME:-$HOME/.local/share}
desktop=$data/applications/minesweeper-deterministic.desktop
icon=$data/icons/hicolor/scalable/apps/minesweeper-deterministic.svg

case "${1:-}" in
    -h|--help) usage; exit 0 ;;
    --uninstall)
        cargo uninstall minesweeper-deterministic || true
        rm -f "$desktop" "$icon"
        echo "Removed."
        exit 0 ;;
    "") ;;
    *) usage >&2; exit 2 ;;
esac

# A build directory of its own, so an install never shares artifacts with
# builds running elsewhere against the same checkout.
cargo install --locked --path "$root" --target-dir "$root/target/install"
bin=${CARGO_HOME:-$HOME/.cargo}/bin/minesweeper-deterministic
install -Dm644 "$root/assets/icon.svg" "$icon"
mkdir -p "$(dirname "$desktop")"
# Launchers often run without ~/.cargo/bin on PATH, so use the full path.
sed "s|^Exec=.*|Exec=$bin|" "$root/assets/minesweeper-deterministic.desktop" > "$desktop"
echo "Installed $bin and a launcher entry."
