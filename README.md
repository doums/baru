## bar

My own bar for [spectrwm](https://github.com/conformal/spectrwm), coded in Rust (and a little bit of C).

![bar](https://image.petitmur.beer/bar.png)

### features

* date and time
* battery (level, status, design level based)
* wireless (state, essid, signal strength, speed)
* audio sink and source (level, muted)
* brightness
* cpu temperature
* cpu usage

Some modules run in their own thread.\
The audio module communicates with the [PulseAudio](https://www.freedesktop.org/wiki/Software/PulseAudio/) server to retrieve its data.\
Wireless module use the [802.11 netlink interface](https://www.infradead.org/~tgr/libnl/) to retrieve its data.

### license
Mozilla Public License 2.0
