/* Real, bounded excerpt of transmission's own libtransmission/quark.cc
 * -- fetched from transmission/transmission @ tag 4.1.3. Contains every
 * real quark string literal that is entirely lowercase/digit/dash (no
 * underscore, no uppercase) -- 278 of 712 total real quarks; the rest
 * are camelCase/snake_case/BT-protocol-specific forms this project's
 * D-extraction deliberately does not need.
 *
 * Real design decision, made after checking (not guessing), and
 * REVISED once during real end-to-end testing:
 *
 * quark.cc's own per-entry trailing comments (e.g. "// tr_session::
 * Settings") look like a clean settings.json-relevance filter at a
 * glance, but are NOT reliable for this purpose -- `rpc-bind-address`/
 * `rpc-port`/`watch-dir`/`watch-dir-enabled` are all real, live keys in
 * a real Nix-evaluated settings.json (confirmed via `nix eval`), yet
 * their own real comments say "rpc server settings"/"daemon, gtk app,
 * qt app", never "tr_session::Settings" -- filtering on that tag would
 * have silently DROPPED 4 real accepted keys.
 *
 * The first real design (kebab-only, i.e. "contains a dash") caught
 * that gap but introduced a NEW one, found for real by the actual
 * `real_transmission_end_to_end_is_a_clean_pass` test failing on its
 * first run: `umask` is a real, live settings.json key with NO dash at
 * all (a single word can't be kebab-vs-snake-cased, since there's no
 * separator to case) -- a "contains a dash" filter silently excludes
 * every single-word key. Broadened to "entirely lowercase/digit/dash,
 * no underscore, no uppercase" instead, which covers both real shapes
 * (kebab multi-word AND bare single-word) while still excluding every
 * real snake_case/camelCase alias sibling.
 */
    "activity-date"sv, // .resume
    "added"sv, // BEP0011; BT protocol, rpc
    "added-date"sv, // .resume
    "added6"sv, // BEP0011; BT protocol
    "address"sv, // rpc
    "alt-speed-down"sv, // gtk app, rpc, speed settings
    "alt-speed-enabled"sv, // gtk app, rpc, speed settings
    "alt-speed-time-begin"sv, // rpc, speed settings
    "alt-speed-time-day"sv, // rpc, speed settings
    "alt-speed-time-enabled"sv, // rpc, speed settings
    "alt-speed-time-end"sv, // rpc, speed settings
    "alt-speed-up"sv, // gtk app, rpc, speed settings
    "announce"sv, // BEP0003; BT protocol
    "announce-ip"sv, // tr_session::Settings
    "announce-ip-enabled"sv, // tr_session::Settings
    "announce-list"sv, // BEP0012; BT protocol
    "anti-brute-force-enabled"sv, // rpc, rpc server settings
    "anti-brute-force-threshold"sv, // rpc server settings
    "arguments"sv, // json-rpc
    "availability"sv, // rpc
    "bandwidth-priority"sv, // .resume
    "bind-address-ipv4"sv, // daemon, tr_session::Settings
    "bind-address-ipv6"sv, // daemon, tr_session::Settings
    "bitfield"sv, // .resume
    "blocklist-date"sv, // gtk app, qt app
    "blocklist-enabled"sv, // daemon, gtk app, rpc, tr_session::Settings
    "blocklist-size"sv, // rpc
    "blocklist-update"sv, // rpc
    "blocklist-updates-enabled"sv, // gtk app, qt app
    "blocklist-url"sv, // rpc, tr_session::Settings
    "blocks"sv, // .resume
    "cache-size-mb"sv, // rpc, tr_session::Settings
    "code"sv, // json-rpc
    "comment"sv, // .torrent, rpc
    "compact-view"sv, // gtk app, qt app
    "complete"sv, // BEP0048; BT protocol
    "config-dir"sv, // rpc
    "cookies"sv, // rpc
    "corrupt"sv, // .resume
    "creator"sv, // rpc
    "cumulative-stats"sv, // rpc
    "current-stats"sv, // rpc
    "data"sv, // json-rpc, rpc
    "default-trackers"sv, // daemon, rpc, tr_session::Settings
    "delete-local-data"sv, // rpc
    "destination"sv, // .resume
    "details-window-height"sv, // gtk app
    "details-window-width"sv, // gtk app
    "dht-enabled"sv, // daemon, rpc, tr_session::Settings
    "dnd"sv, // .resume
    "done-date"sv, // .resume
    "download-dir"sv, // daemon, gtk app, tr_session::Settings
    "download-dir-free-space"sv, // rpc
    "download-queue-enabled"sv, // rpc, tr_session::Settings
    "download-queue-size"sv, // rpc, tr_session::Settings
    "downloaded"sv, // BEP0048; .resume, BT protocol
    "downloaded-bytes"sv, // stats.json
    "downloading-time-seconds"sv, // .resume
    "dropped"sv, // BEP0011; BT protocol
    "dropped6"sv, // BEP0011; BT protocol
    "e"sv, // BT protocol
    "encoding"sv, // .torrent
    "encryption"sv, // daemon, rpc, tr_session::Settings
    "error"sv, // rpc
    "eta"sv, // rpc
    "fields"sv, // rpc
    "file-count"sv, // rpc
    "filename"sv, // rpc
    "files"sv, // .resume, .torrent, rpc
    "files-added"sv, // stats.json
    "files-unwanted"sv, // rpc
    "files-wanted"sv, // rpc
    "filter-mode"sv, // qt app
    "filter-text"sv, // qt app
    "filter-trackers"sv, // qt app
    "flags"sv, // .resume
    "format"sv, // rpc
    "free-space"sv, // rpc
    "group"sv, // .resume, rpc
    "group-get"sv, // rpc
    "group-set"sv, // rpc
    "host"sv, // rpc
    "id"sv, // dht.dat, rpc
    "idle-limit"sv, // .resume
    "idle-mode"sv, // .resume
    "idle-seeding-limit"sv, // rpc, tr_session::Settings
    "idle-seeding-limit-enabled"sv, // rpc, tr_session::Settings
    "ids"sv, // rpc
    "incomplete"sv, // BEP0048; BT protocol
    "incomplete-dir"sv, // .resume, daemon, gtk app, rpc, tr_session::Settings
    "incomplete-dir-enabled"sv, // daemon, rpc, tr_session::Settings
    "info"sv, // .torrent
    "inhibit-desktop-hibernation"sv, // gtk app, qt app
    "ipv4"sv, // BEP0010; BT protocol, rpc
    "ipv6"sv, // BEP0010; BT protocol, rpc
    "jsonrpc"sv, // json-rpc
    "labels"sv, // .resume, rpc
    "length"sv, // .torrent, rpc
    "location"sv, // rpc
    "lpd-enabled"sv, // daemon, rpc, tr_session::Settings
    "m"sv, // BEP0010, BEP0011; BT protocol
    "main-window-height"sv, // gtk app, qt app
    "main-window-is-maximized"sv, // gtk app
    "main-window-layout-order"sv, // qt app
    "main-window-width"sv, // gtk app, qt app
    "main-window-x"sv, // gtk app, qt app
    "main-window-y"sv, // gtk app, qt app
    "max-peers"sv, // .resume
    "memory-bytes"sv, // rpc
    "memory-units"sv, // rpc
    "message"sv, // json-rpc, rpc
    "message-level"sv, // daemon, gtk app, tr_session::Settings
    "metainfo"sv, // rpc
    "method"sv, // json-rpc
    "move"sv, // rpc
    "mtimes"sv, // .resume
    "name"sv, // .resume, .torrent, rpc
    "nodes"sv, // dht.dat
    "nodes6"sv, // dht.dat
    "open-dialog-dir"sv, // gtk app, qt app
    "p"sv, // BEP0010; BT protocol
    "params"sv, // json-rpc
    "path"sv, // .torrent, rpc
    "paused"sv, // .resume, rpc
    "peer-congestion-algorithm"sv, // tr_session::Settings
    "peer-limit"sv, // rpc
    "peer-limit-global"sv, // daemon, rpc, tr_session::Settings
    "peer-limit-per-torrent"sv, // daemon, gtk app, rpc, tr_session::Settings
    "peer-port"sv, // daemon, gtk app, rpc, tr_session::Settings
    "peer-port-random-high"sv, // tr_session::Settings
    "peer-port-random-low"sv, // tr_session::Settings
    "peer-port-random-on-start"sv, // rpc, tr_session::Settings
    "peer-socket-tos"sv, // tr_session::Settings
    "peers"sv, // rpc
    "peers2"sv, // .resume
    "peers2-6"sv, // .resume
    "pex-enabled"sv, // rpc, tr_session::Settings
    "pidfile"sv, // daemon
    "piece"sv, // BT protocol
    "pieces"sv, // .resume, .torrent, rpc
    "port"sv, // rpc
    "port-forwarding-enabled"sv, // daemon, rpc, tr_session::Settings
    "port-is-open"sv, // rpc
    "port-test"sv, // rpc
    "preallocation"sv, // tr_session::Settings
    "primary-mime-type"sv, // rpc
    "priorities"sv, // rpc
    "priority"sv, // .resume, rpc
    "priority-high"sv, // rpc
    "priority-low"sv, // rpc
    "priority-normal"sv, // rpc
    "private"sv, // .torrent
    "progress"sv, // .resume, rpc
    "prompt-before-exit"sv, // qt app
    "queue-move-bottom"sv, // rpc
    "queue-move-down"sv, // rpc
    "queue-move-top"sv, // rpc
    "queue-move-up"sv, // rpc
    "queue-stalled-enabled"sv, // rpc, tr_session::Settings
    "queue-stalled-minutes"sv, // rpc, tr_session::Settings
    "ratio-limit"sv, // .resume, daemon, gtk app, tr_session::Settings
    "ratio-limit-enabled"sv, // daemon, tr_session::Settings
    "ratio-mode"sv, // .resume
    "read-clipboard"sv, // qt app
    "recently-active"sv, // rpc
    "remote-session-enabled"sv, // qt app
    "remote-session-host"sv, // qt app
    "remote-session-https"sv, // qt app
    "remote-session-password"sv, // qt app
    "remote-session-port"sv, // qt app
    "remote-session-requres-authentication"sv, // SIC: misspelled prior to 4.1.0-beta.4; qt app
    "remote-session-username"sv, // qt app
    "removed"sv, // rpc
    "rename-partial-files"sv, // rpc, tr_session::Settings
    "reqq"sv, // BEP0010; BT protocol, rpc, tr_session::Settings
    "result"sv, // rpc
    "rpc-authentication-required"sv, // daemon, rpc server settings
    "rpc-bind-address"sv, // daemon, rpc server settings
    "rpc-enabled"sv, // daemon, rpc server settings
    "rpc-host-whitelist"sv, // rpc, rpc server settings
    "rpc-host-whitelist-enabled"sv, // rpc, rpc server settings
    "rpc-password"sv, // daemon, rpc server settings
    "rpc-port"sv, // daemon, gtk app, rpc server settings
    "rpc-socket-mode"sv, // rpc server settings
    "rpc-url"sv, // rpc server settings
    "rpc-username"sv, // daemon, rpc server settings
    "rpc-version"sv, // rpc
    "rpc-version-minimum"sv, // rpc
    "rpc-version-semver"sv, // rpc
    "rpc-whitelist"sv, // daemon, gtk app, rpc server settings
    "rpc-whitelist-enabled"sv, // daemon, rpc server settings
    "scrape"sv, // rpc
    "scrape-paused-torrents-enabled"sv, // tr_session::Settings
    "script-torrent-added-enabled"sv, // rpc, tr_session::Settings
    "script-torrent-added-filename"sv, // rpc, tr_session::Settings
    "script-torrent-done-enabled"sv, // rpc, tr_session::Settings
    "script-torrent-done-filename"sv, // rpc, tr_session::Settings
    "script-torrent-done-seeding-enabled"sv, // rpc, tr_session::Settings
    "script-torrent-done-seeding-filename"sv, // rpc, tr_session::Settings
    "seconds-active"sv, // stats.json
    "seed-queue-enabled"sv, // rpc, tr_session::Settings
    "seed-queue-size"sv, // rpc, tr_session::Settings
    "seeding-time-seconds"sv, // .resume
    "session-close"sv, // rpc
    "session-count"sv, // stats.json
    "session-get"sv, // rpc
    "session-id"sv, // rpc
    "session-set"sv, // rpc
    "session-stats"sv, // rpc
    "show-backup-trackers"sv, // gtk app, qt app
    "show-extra-peer-details"sv, // gtk app
    "show-filterbar"sv, // gtk app, qt app
    "show-notification-area-icon"sv, // gtk app, qt app
    "show-options-window"sv, // gtk app, qt app
    "show-statusbar"sv, // gtk app, qt app
    "show-toolbar"sv, // gtk app, qt app
    "show-tracker-scrapes"sv, // gtk app, qt app
    "sitename"sv, // rpc
    "size-bytes"sv, // rpc
    "size-units"sv, // rpc
    "sleep-per-seconds-during-verify"sv, // tr_session::Settings
    "sort-mode"sv, // gtk app, qt app
    "sort-reversed"sv, // gtk app, qt app
    "source"sv, // .torrent
    "speed"sv, // .resume
    "speed-bytes"sv, // rpc
    "speed-limit-down"sv, // .resume, gtk app, rpc, tr_session::Settings
    "speed-limit-down-enabled"sv, // rpc, tr_session::Settings
    "speed-limit-up"sv, // .resume, gtk app, rpc, tr_session::Settings
    "speed-limit-up-enabled"sv, // rpc, tr_session::Settings
    "speed-units"sv, // rpc
    "start-added-torrents"sv, // gtk app, rpc, tr_session::Settings
    "start-minimized"sv, // qt app
    "status"sv, // rpc
    "statusbar-stats"sv, // gtk app, qt app
    "tag"sv, // rpc
    "tcp-enabled"sv, // rpc, tr_session::Settings
    "tier"sv, // rpc
    "time-checked"sv, // .resume
    "torrent-add"sv, // rpc
    "torrent-added"sv, // rpc
    "torrent-added-notification-enabled"sv, // gtk app, qt app
    "torrent-added-verify-mode"sv, // tr_session::Settings
    "torrent-complete-notification-enabled"sv, // gtk app, qt app
    "torrent-complete-sound-command"sv, // gtk app, qt app
    "torrent-complete-sound-enabled"sv, // gtk app, qt app
    "torrent-duplicate"sv, // rpc
    "torrent-get"sv, // rpc
    "torrent-reannounce"sv, // rpc
    "torrent-remove"sv, // rpc
    "torrent-rename-path"sv, // rpc
    "torrent-set"sv, // rpc
    "torrent-set-location"sv, // rpc
    "torrent-start"sv, // rpc
    "torrent-start-now"sv, // rpc
    "torrent-stop"sv, // rpc
    "torrent-verify"sv, // rpc
    "torrents"sv, // rpc
    "trackers"sv, // rpc
    "trash-can-enabled"sv, // gtk app
    "trash-original-torrent-files"sv, // gtk app, rpc, tr_session::Settings
    "umask"sv, // tr_session::Settings
    "units"sv, // rpc
    "upload-slots-per-torrent"sv, // tr_session::Settings
    "uploaded"sv, // .resume
    "uploaded-bytes"sv, // stats.json
    "url-list"sv, // .torrent
    "use-global-speed-limit"sv, // .resume
    "use-speed-limit"sv, // .resume
    "utp-enabled"sv, // daemon, rpc, tr_session::Settings
    "v"sv, // BEP0010; BT protocol
    "version"sv, // rpc
    "wanted"sv, // rpc
    "watch-dir"sv, // daemon, gtk app, qt app
    "watch-dir-enabled"sv, // daemon, gtk app, qt app
    "watch-dir-force-generic"sv, // daemon
    "webseeds"sv, // rpc
    "yourip"sv, // BEP0010; BT protocol
