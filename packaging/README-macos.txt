CRIMSON  v1.0
A warm metallic vocal chorus, built to sit behind pitch correction.
Pyfessional  ·  https://pyfessional.tech


INSTALL
-------
1. Copy Crimson.vst3 into:
       ~/Library/Audio/Plug-Ins/VST3/

   and/or Crimson.clap into:
       ~/Library/Audio/Plug-Ins/CLAP/

2. Restart your DAW and rescan plugins.


IF MACOS SAYS THE PLUGIN IS DAMAGED
-----------------------------------
If you see "Crimson.vst3 is damaged and can't be opened" or
"cannot be opened because Apple cannot check it for malicious
software" — the file is fine. That is Gatekeeper.

Crimson is not yet code-signed with an Apple Developer ID, so
macOS quarantines it after download. Clear the flag in Terminal:

    xattr -rd com.apple.quarantine ~/Library/Audio/Plug-Ins/VST3/Crimson.vst3
    xattr -rd com.apple.quarantine ~/Library/Audio/Plug-Ins/CLAP/Crimson.clap

Run only the line for the format you installed. Then rescan in
your DAW.

If you would rather not run that, the full source is public and
you can build it yourself — link at the bottom.


COMPATIBILITY
-------------
Formats:  VST3, CLAP
Works in: Ableton Live, FL Studio, Reaper, Bitwig, Studio One,
          Cubase, and other VST3 or CLAP hosts.

There is no Audio Unit build, so Crimson will NOT appear in
Logic Pro or GarageBand.


GETTING THE INTENDED SOUND
--------------------------
The defaults are voiced, not arbitrary. They were set on real
tuned vocals:

    Rate      1.60 Hz
    Depth     4.1 ms
    Feedback  44%
    Mix       40%

Start there. Feedback above roughly 70% is deliberately allowed
to get resonant and strange — that is character, not a bug, but
it is not where the plugin was voiced.


SOURCE, BUGS, QUESTIONS
-----------------------
https://github.com/MichaelAngulo3232/crimson-chorus
contact@pyfessional.tech


LICENSE
-------
GPLv3. See LICENSE in the source repository.
