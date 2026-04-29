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

# Background HTTP/TLS traffic so dashboards have something to show; launch netwatch with capture privileges.
Hide
Type "bash -lc 'while true; do curl -I http://example.com >/dev/null 2>&1; curl -I https://example.com >/dev/null 2>&1; sleep 0.8; done & echo $NETWATCH_DEMO_PASSWORD | sudo -S ./target/release/netwatch'"
Enter
Sleep 6s
Show

# Dashboard — KPIs, throughput sparkline filling the full window, top connections.
Sleep 5s

# Connections — filter chips, real-time RX/TX, sortable columns.
Type "2"
Sleep 3s

# Topology — v0.14 redesign: locals on the left, public peers on the right,
# health dots on the trunks to ROUTER and ISP. Auto-traceroute fills the hop
# table without a keypress.
Type "6"
Sleep 4s

# Processes — per-process kernel RTT and CPU% with rolling history.
Type "8"
Sleep 3s

# Timeline — RTT spike and interface flap detectors as discrete events.
Type "7"
Sleep 3s

# Back to dashboard for the loop tail.
Type "1"
Sleep 2s

Hide
Type "q"
Sleep 500ms
