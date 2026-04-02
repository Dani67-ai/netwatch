Output demo.gif

Require curl

Set Shell "bash"
Set FontSize 14
Set Width 1200
Set Height 700
Set Theme "Catppuccin Mocha"
Set TypingSpeed 30ms
Set Framerate 18
Set Padding 10

# Launch hidden: generate HTTP/TLS traffic in the background and start NetWatch with capture privileges
Hide
Type "bash -lc 'while true; do curl -I http://example.com >/dev/null 2>&1; curl -I https://example.com >/dev/null 2>&1; sleep 0.8; done & echo $NETWATCH_DEMO_PASSWORD | sudo -S ./target/release/netwatch'"
Enter
Sleep 4s
Show

# Dashboard: instant visibility
Sleep 2s

# Arm the flight recorder, but keep focus on the whole product
Type "R"
Sleep 1s

# Connections: who owns the traffic?
Type "2"
Sleep 2s

# Packets: live capture, stream view, display filter, pcap export
Type "4"
Sleep 3s
Down 2
Sleep 300ms
Enter
Sleep 500ms
Type "s"
Sleep 2s
Type "h"
Sleep 1s
Escape
Sleep 500ms
Type "/"
Sleep 300ms
Type "dns"
Enter
Sleep 1500ms
Escape
Sleep 500ms
Type "w"
Sleep 1500ms

# Topology: network map and remote hosts
Type "6"
Sleep 2500ms

# Processes: which apps are driving traffic?
Type "8"
Sleep 2500ms

# Back to dashboard and freeze/export the captured incident window
Type "1"
Sleep 1500ms
Type "F"
Sleep 800ms
Type "E"
Sleep 2s

Type "q"
Sleep 500ms
