[![baru](https://img.shields.io/github/actions/workflow/status/doums/baru/test.yml?color=0D0D0D&logoColor=BFBFBF&labelColor=404040&logo=github&style=for-the-badge)](https://github.com/doums/baru/actions?query=workflow%3ATest)
[![baru](https://img.shields.io/aur/version/baru?color=0D0D0D&logoColor=BFBFBF&labelColor=404040&logo=arch-linux&style=for-the-badge)](https://aur.archlinux.org/packages/baru/)

## baru

A simple system monitor for WM statusbar

![baru](https://raw.githubusercontent.com/doums/baru/master/public/baru.png)

Baru is a lightweight system monitor for WM status-bar.\
It can be used as a provider with any status-bar that can read from `stdout`.\
Like [xmobar](https://codeberg.org/xmobar/xmobar),
[lemonbar](https://github.com/LemonBoy/bar),
[dwm](https://dwm.suckless.org/status_monitor/) etc…

---

[features](#features) ∎ [prerequisite](#prerequisite) ∎ [install](#install) ∎ [configuration](#configuration) ∎ [usage](#usage) ∎ [credits](#credits) ∎ [license](#license)

### features

* date and time
* battery (level, status, design level based)
* wireless (state, essid, signal strength)
* wired (state)
* audio sink and source (level, muted)
* brightness
* cpu usage, frequency and temperature
* memory (percent or used/total in gigabyte/gibibyte)
* weather current condition and
  temperature ([OpenWeatherMap](https://openweathermap.org/))
* dynamic and customizable labels, play nicely with icons and [nerd-fonts](https://www.nerdfonts.com/)
* customizable format output
* configuration in YAML

### prerequisite

The following system libraries are required:

- libnl (for wired and wireless modules)
- libpulse (for sound and mic modules)

### install

- Arch Linux (AUR) [package](https://aur.archlinux.org/packages/baru)
- latest [release](https://github.com/doums/baru/releases)

### configuration

The binary looks for the config file `baru.yaml` located
in `$XDG_CONFIG_HOME/baru/` (default to `$HOME/.config/baru/`).\
If the config file is not found, baru prints an error and exits.\
All options are
detailed [here](https://github.com/doums/baru/blob/master/baru.yaml).

TIPS: To test and debug your config run baru from the terminal like this:

```shell
RUST_LOG=debug baru -l stdout
```

#### Config example

```yaml
format: '%m  %f  %c  %t  %b  %i  %s   %w%e  %a    %d'
tick: 50
battery:
  full_design: true
  low_level: 30
  full_label: '*'
  charging_label: '^'
  discharging_label: 'b'
  low_label: '!'
  unknown_label: '?'
  format: '%l %v'
brightness:
  label: 'l'
  format: '%l %v'
cpu:
  label: 'c'
  high_label: '!'
  format: '%v %l'
cpu_freq:
  tick: 100
  high_level: 60
  label: 'f'
  high_label: '!'
  format: '%v %l'
memory:
  label: 'm'
  high_label: '!'
  format: '%v %l'
mic:
  label: 'i'
  mute_label: '.'
  format: '%v %l'
sound:
  label: 's'
  mute_label: '.'
  format: '%v %l'
temperature:
  core_inputs: 2..5
  label: 't'
  high_label: '!'
  format: '%v %l'
wired:
  discrete: true
  label: 'e'
  disconnected_label: '\'
  format: '%l'
wireless:
  interface: wlan0
  display: Essid
  max_essid_len: 5
  label: 'w'
  disconnected_label: '\'
  format: '%v %l'
weather:
  tick: 300 # seconds
  # your openweathermap api key
  api_key: 1234567890
  location: 'Metz'
  unit: metric
  icons:
    clear_sky: [ '󰖙', '󰖔' ] # day, night
    partly_cloudy: [ '󰖕', '󰼱' ]
    cloudy: '󰖐'
    very_cloudy: '󰖐'
    shower_rain: '󰖖'
    rain: '󰖖'
    thunderstorm: '󰖓'
    snow: '󰖘'
    mist: '󰖑'
  format: '%v'
```

### usage

```shell
baru -h
```

When spawning baru from your WM/status-bar you can pass the `-l file` flag\
if you want baru to log into a file (useful for debugging).\
Logs are written to the directory `$XDG_CACHE_HOME/baru/` (default
to `$HOME/.cache/baru/`).

```shell
baru -l file
```

### implementation details

Baru gathers the information from `/sys` and `/proc` filesystems (filled by the
kernel).\
Except audio and network modules which use C libraries.\
All modules are threaded and loaded on-demand.\
Thanks to this modular design (as well Rust and C), baru is lightweight and
efficient.\
It can run at high refresh rate with a minimal cpu footprint.

The audio module communicates with
the [PipeWire](https://pipewire.org/)/[PulseAudio](https://www.freedesktop.org/wiki/Software/PulseAudio/)\
server
through [client API](https://freedesktop.org/software/pulseaudio/doxygen/) to
retrieve its data. Wireless and wired\
modules use the netlink interface with the help
of [libnl](https://www.infradead.org/~tgr/libnl/) to talk directly\
to kernel and retrieve their data.\
In addition, wireless module uses
the [802.11](https://github.com/torvalds/linux/blob/master/include/uapi/linux/nl80211.h)
API.

### dev

#### prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- CMake
- libnl and libpulse present on the system

```shell
RUST_LOG=trace cargo run -- -l stdout
```

### credits

Clément Dommerc for providing me with the C code for the lib `netlink`, wireless
part.

### license

Mozilla Public License 2.0
