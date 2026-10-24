# Future Improvements Ruled Out

Ideas from [future-improvements.md](future-improvements.md) that were looked into and set aside: what was tried, why it stopped, and what would reopen it. They're kept so the same ground isn't covered twice.

---

### Sampling once per meter update

At Every reading (0 ms) the stream keeps each frame as it arrives, so it follows a meter's frame rate exactly; the trouble is a meter that updates its reading more slowly than it sends frames. A UT181A sends a frame every 100 ms but changes its main reading every 500 to 700 ms depending on the mode, so each reading lands five to seven times. A polled meter asked back to back does the same: a UT61E+ answers every 97 ms, and its digits change at a cadence set by the mode, every 300 ms in V, Ω and capacitance and every 400 ms in DC mA ([UT61E+ spec §2.8](research/ut61eplus/reverse-engineered-protocol.md#28-sampling-rate--verified)). A Sample interval of the update period comes close, but its ticks are not aligned with the meter's updates, so a reading near a tick can be kept twice or skipped.

No protocol we support marks a new reading: none found carries a sequence counter or an update flag, and the frames between updates are not reliably identical. A UT181A's fast sample changes on every frame, and a UT61E+'s bar graph often moves while its digits hold. Even where the frames are identical, a new reading of the same value arrives as the same bytes. Each candidate signal has a gap. A BM78xBT reading packet carries the meter's clock to the millisecond, but whether it advances per reading or per notification is unknown. Manuals give one nominal figure or a range per meter (the UT181A's 2 Sa/s at 60K counts; 2–3 Hz on most others), while the period changes with the mode: the UT61E+ cadences above, and every 700–900 ms on the UT181A's temperature dial. Inferring the period from value changes fails on a steady input, where the repeats keep coming with no change to lock onto ([a live refresh-rate estimate](#a-live-refresh-rate-estimate)). The graph's gap detector and the Buffer size estimate assume today's frame rate and would have to change with any design.

Use cases: a CSV whose rows are the meter's readings, not its frames. Raised by @diego351's UT181A at 2 Sa/s ([issue #5](https://github.com/antoinecellerier/dmm-tools/issues/5)).

Reopens if: a protocol is found to mark each new reading, such as the BM78xBT's clock advancing once per reading.

### A live refresh-rate estimate

A Statistics row and a line in `dmm-cli read`'s closing summary giving how often the meter updates its reading, such as "≈ 300 ms ±0.2", estimated from the times its digits change. A test for a fixed beat in those times, with its false-alarm threshold calibrated on simulated meters, matched the beats fitted by hand to the recordings (UT61E+ 300 and 400 ms, UT161E 300 ms, UT181A 500 to 700 ms by mode) and declined on Bluetooth delivery and on meters with no steady pattern.

It was set aside because it only answers while the input changes: a steady input leaves too few changes, and short capture steps often ended without an answer. The answer is also close to a constant per meter and mode, now recorded in the family specs ([UT61E+ §2.8](research/ut61eplus/reverse-engineered-protocol.md#28-sampling-rate--verified), [UT161E §2.2](research/ut61-family/reverse-engineered-protocol.md#22-ut61-vs-ut161--vendor), [UT181A §5.1](research/ut181/reverse-engineered-protocol.md#51-common-header-5-bytes)), and it cannot drive sampling once per update, which needs the beat before the input moves. On simulated meters whose updates came 280–330 ms apart with no fixed phase, it still reported a period, with too narrow a ±.

Reopens if: someone needs the figure live, such as to spot a meter updating more slowly than its family spec says.
