# Herdr 0.9 popup startup delay

On 2026-09-16, client and server 0.9.0 reproduced a 150 ms cell-size timeout on
three consecutive real popup launches. Warm launch-to-measure times were 174 and
171 ms; the picker fell back to 10×20 px instead of the host’s 16×38 px.

This is the same geometry-reconciliation omission as Flash’s `layout.apply`
regression, through a different request. In Herdr 0.9.0,
`src/server/headless/client_views.rs::public_request_may_change_geometry` excludes
`PluginPaneOpen`. Popup creation reconciles ownership, but does not reapply the
controlling client’s geometry to the fresh runtime.

Before issuing CSI 16 t, Hop now sends `pane.resize` on a real target pane with
`direction: right` and explicit `amount: 0.0`. This triggers geometry reconciliation
without changing split ratios or focus. The popup then answers the existing PTY
query. Failure remains best-effort, preserving the old query and silent fallback.
Do not replace the self-query with global `pane.graphics.info`: the self-query is
needed for hosts whose global graphics geometry is not yet initialized.

A temporary diagnostic plugin executed the actual picker inside a 6×4 popup,
recorded query duration to a file, and exited through normal overlay cleanup.
With only the zero-delta resize added, all three queries completed in 1 ms with
16×38 px; warm launch-to-measure times were 29 and 23 ms. First-launch samples
included roughly 0.6 seconds of binary cold-start overhead and are excluded from
the warm comparison. These timings measure through cell-size acquisition, not
physical display presentation or the full prefix key dispatch.

Regression coverage exercises the picker preparation path against a socket
server that releases the PTY report only after the exact zero-delta resize request.
Without the fix it returns the fallback size; with the fix it reads the host size.
A second test verifies that failed refresh does not prevent PTY measurement.
