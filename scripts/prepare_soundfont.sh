#!/usr/bin/env bash
# Build Bratdog's bank, assets/soundfonts/bratdog.sf2, and the LICENSE beside it, through
# the engine's scripts/prepare_game_soundfont.sh: the FluidR3 GM presets the Tritsch-Tratsch
# Polka plays. The subset is committed; this script is how anyone rebuilds it byte for byte.
#
# Usage: scripts/prepare_soundfont.sh [--bank path/to/FluidR3_GM.sf2]
#   The engine's script documents the fetch, the sha256 check and the 4 MiB cap.
#
# The presets, as bank:program: accordion (the polka's tune, Frank's snacks, the jam),
# clarinet (the counter-melody, and the second dog's snacks on the same tune), tuba (the
# oom), trombone (the pah, and a death's blat), woodblock (a paw's click on a turn). No
# strings and no kit: the accordion is two thirds of the bank, and the bank stays under 4 MiB.
set -euo pipefail

GAME_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ENGINE_DIR="$(cd "$GAME_DIR/../../insiculous_2d" && pwd)"
OUTPUT_DIR="$GAME_DIR/assets/soundfonts"

exec "$ENGINE_DIR/scripts/prepare_game_soundfont.sh" "$@" --max-mib 4 "$OUTPUT_DIR" bratdog 0:21 0:71 0:58 0:57 0:115
