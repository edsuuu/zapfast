# Notification sounds

`pop.wav` (new messages, the default) and `pop-mention.wav` (mentions and
replies to us) are ZapFast's own, in the style of WhatsApp's notification
tone: short bubble pops synthesized from sine waves by
`scripts/generate-pop-sounds.py`. They copy no recording and are MIT-licensed
like the rest of ZapFast. Run the script again after changing a note.

`receive.wav` (one-to-one chats) and `alert.wav` (groups) are Pidgin's
message sounds, copied unmodified from `share/sounds/` in the Pidgin 2.14.14
source release (<https://pidgin.im>). Pidgin distributes them under the GNU
General Public License, version 2; the full text is in `THIRD-PARTY-NOTICES.md` at the repository root.
They remain under that license. The rest of ZapFast is MIT-licensed; these
two files are separate data files that ZapFast plays, not part of its code.
