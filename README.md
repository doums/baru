## bar

My own bar for [spectrwm](https://github.com/conformal/spectrwm), coded in Rust (and a little bit of C).

![bar](https://image.petitmur.beer/bar.png)

### features

* date and time
* battery (level, status, design level based)
* wireless (state, essid, signal strength, speed)
* audio sink and source (level, muted)
* brightness
* cpu usage and temperature
* icons, colors

All the info is read direclty from the kernel files (`/sys`, `/proc/stat`). Except for the audio and wireless modules.\
The audio module communicates with the [PulseAudio](https://www.freedesktop.org/wiki/Software/PulseAudio/) server through the [client API](https://freedesktop.org/software/pulseaudio/doxygen/) to retrieve its data.\
Wireless module use the [802.11 netlink interface](https://www.infradead.org/~tgr/libnl/) to retrieve its data.

Some modules run in their own thread.

### license
Mozilla Public License 2.0
