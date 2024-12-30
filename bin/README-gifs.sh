#!/use/bin/env bash
#
# SPDX-FileCopyrightText: 2024 antlers <antlers@illucid.net>
# SPDX-License-Identifier: GPL-3.0-or-later
#
# To be ran from project root.

set -xeuo pipefail

[[ -f "$1" ]] || printf 'error: %s' "File \"$1\" not found."
./bin/3D.py -AS -o ./img/score.gif "$1"
./bin/3D.py -A -o ./img/density.gif "$"
gifsicle -i --lossy=30 --scale 0.3 ./img/density.gif -O3 --colors 148 -o ./img/density-opt.gif
gifsicle -i --lossy=30 --scale 0.3 ./img/score.gif -O3 --colors 148 -o ./img/score-opt.gif
rm ./img/score.gif
rm ./img/density.gif
git add ./img/score-opt.gif ./img/density-opt.gif
# git commit --amend --no-edit && git push -f
