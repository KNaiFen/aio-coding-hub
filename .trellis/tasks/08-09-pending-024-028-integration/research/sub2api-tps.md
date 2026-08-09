# sub2api TPS Reference

- Repository: `Wei-Shaw/sub2api`
- Pinned comparison commit: `00b8596176809906993169c283671811ad04f58d`
- Current-main comparison during planning: `cc67b1aca1d3b590609abef2fcd3a6ca31c5c651`
- Formula: `AVG(output_tokens * 1000.0 / duration_ms)` for rows with positive output and duration.
- `duration_ms` covers the complete successful `Forward` attempt and includes first-token latency; prior failed attempts are not part of the successful usage row.
- AIO therefore uses `final_upstream_attempt_duration_ms`, never `duration_ms - ttfb_ms`, and combines rows with a rate sum/sample count rather than token/duration sums.
- AIO database sampling found successful client-disconnect rows with output usage but `timing_version=0`: downstream closure invalidated timing even when upstream drain later observed a trustworthy protocol completion. The implementation must separate output-stream validity from final-attempt validity.
