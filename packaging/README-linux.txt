CRIMSON  __VERSION__
A warm metallic vocal chorus, built to sit behind pitch correction.
Pyfessional  ·  https://pyfessional.tech


INSTALL
-------
1. Copy Crimson.vst3 into:
       ~/.vst3/

   and/or Crimson.clap into:
       ~/.clap/

   Create the directory first if it does not exist.

2. Restart your DAW and rescan plugins.


DEPENDENCIES
------------
Built against ALSA and JACK. If the plugin fails to load, make
sure libasound2 and a working X11 or Wayland session are present.


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
