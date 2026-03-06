# What LoopForge Is Borrowing Next from OpenFang and OpenClaw

## TL;DR

This iteration does not try to copy the full surface area of OpenFang or OpenClaw.
Instead, LoopForge borrows the parts that matter most to first-run success:

- clearer onboarding
- better troubleshooting
- more obvious first-day tasks

## What OpenFang does well

OpenFang is strong at turning capability into visible “what can I do with this?” framing.
The template/catalog mentality helps new users reach useful outcomes faster.

What LoopForge borrows now:

- starter-task framing
- tighter getting-started path
- clearer next-step guidance after setup

## What OpenClaw does well

OpenClaw is strong at help, testing, troubleshooting, and operator trust signals.
That matters because a large part of first-run frustration is not missing features — it is not knowing what failed and what to do next.

What LoopForge borrows now:

- stronger troubleshooting entrypoints
- more actionable doctor guidance
- onboarding reports that summarize the last run

## Why LoopForge is not copying everything

LoopForge is optimized for a different core job:

- engineering delivery
- reproducible work
- file artifacts and checkpoints
- local-first workflows

That means this iteration stays focused on the shortest path from install to useful engineering output.

## What changed in this iteration

- `loopforge onboard` now acts more like a guided first-run entrypoint
- onboarding writes `.loopforge/onboard-report.json` and `.md` into the workspace
- `loopforge doctor` gives clearer suggested next steps
- docs now route new users through onboarding, starter tasks, and troubleshooting more directly

## Decision rule

If a competitor feature helps users reach first success faster **without** pulling LoopForge away from its engineering-delivery focus, it is a good candidate for adoption.
