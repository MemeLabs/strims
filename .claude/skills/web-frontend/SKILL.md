---
name: web-frontend
description: The web UI under src/ (components, pages, the watch page, the devtools app) and its tooling. Use when changing UI code, installing or upgrading npm dependencies, writing or fixing storybook stories, running or reading the visual-regression compare (hack/storybook/check.py), or changing the UI's CI workflows. For src/apis and anything under schema/, use proto-bindings.
---

# The web frontend

React and TypeScript, built by webpack (webpack.config.js, swc):
`src/web` boots the app and its worker node, `src/components` and
`src/pages` are its UI, `src/watch` is the watch page, and `src/devtools`
is the devtools app (the sim console, the storybook). The README's "web
ui" section starts the dev server (`pnpm run dev:web`).

## Installing

- pnpm runs at package.json's pinned version and Node at .nvmrc's. Each
  worktree runs its own `pnpm install --frozen-lockfile`, which is cheap.
- The main checkout's install runs only when landing a change to
  package.json or pnpm-lock.yaml, after checking that no build, watch or
  server runs from it. The editor's oxc (`oxlint --lsp`) and TypeScript
  extensions use that checkout's node_modules.
- Upgrade in a worktree (`pnpm add`, `pnpm update`) and commit package.json
  with pnpm-lock.yaml. A dependency with an install script fails the
  install until pnpm-workspace.yaml lists it. ts-key-enum stays on 2.x:
  3.x ships only a const enum, which the isolated-modules build can't use.
  A dependency change gets `--compare main --all-modes`, with every change
  explained.

## Checks for a UI change

    pnpm run typecheck   # tsc
    pnpm run lint        # oxlint src
    pnpm run test:js     # mocha over src/**/*.test.ts

Then run the strict catalog check and the compare below. Prettier
(.prettierrc.json) formats on save. A change to a plugin in webpack/ needs
`pnpm exec tsc -p webpack`, with webpack/lib committed beside it.

## The storybook

`python3 hack/console/server.py --build` serves it at
http://127.0.0.1:8086/devtools.html#/storybook, and /storybook lists the
catalog's covered, skipped and missing files.

- The catalog is every .tsx under src/components, src/pages and
  src/watch. Story modules mirror source paths:
  `src/devtools/stories/components/Chat/Message.stories.tsx` covers
  `src/components/Chat/Message.tsx`. A module's `catalog` export names the
  other files it covers, plus the files that need no story with a reason.
  A new UI file gets its own mirrored module.
- The support module, src/devtools/storybook (types.ts documents a
  story), holds MockApi, StoryRouter, the Static* and Mock* providers and
  fixtures; service mocks are under `src/devtools/mocks/<service>/`. A
  call with no mock, or a mock returning a plain object, fails the story;
  a mock's rejected promise is the call's RPC error. Extend the module,
  not one story.
- Device frames (host/devices.ts) mimic a device's user agent, orientation
  and touch screen. The landscape phone frame also draws an iPhone's notch
  and corners on the side the story's turn (`landscape: "cw"`) puts them,
  and has its safe area. Styles keep writing `env(safe-area-inset-*)`;
  webpack/SafeAreaInsets.ts makes the frame's values reach them.
- A story frame's localStorage is an in-memory stub (host/preload.ts),
  empty for each story, so stories never read or write the origin's.
  Components, hooks and contexts reach local storage only through src/lib
  modules (lib/storage's helpers, lib/devicePreferences). Those modules
  treat a page without storage as one with nothing stored.
- A story fails on a crash or an empty root at `--timeout`. It also fails
  on any error no allow list (`allowErrors`) covers: an uncaught error or
  rejection, or a console.error such as a React warning or a translation
  key that en lacks.
- `python3 hack/storybook/check.py --port N --build --strict` loads every
  story. `--only PREFIX` and `--files PREFIX` narrow it, and
  `--list --strict` checks the catalog alone.

## Visual regression

    python3 hack/storybook/check.py --port N --compare main [--all-modes]

It shoots each story in REF's build (cached under target/storybook/base/)
and this checkout's and diffs them, with an HTML report under
target/storybook/compare/. Run it on every UI change before landing:
`--all-modes` (six modes) for layout, theme or dependency changes, else
one mode (`--theme`, `--device`).

- Manifest scoping captures only the stories whose built inputs differ.
  The rest count as `identical`. `--compare main --list` previews the
  scope and the differing inputs.
- A stale head build or node_modules that don't match the lockfile stop
  the run (exit 2); `--build` rebuilds the head. `--fail-on-diff` exits 1
  on any change, and `--fail-on-error` exits 1 on a head-build failure.
- Captures are deterministic: a build compared with itself diffs to
  nothing, on CI runners too, so a changed story has a cause. A story's
  `visual` (reason, mask, skip, fresh) is for content no capture setting
  holds still. Use it sparingly.
- With `--port`, a run's servers and browsers live in a bwrap box of
  their own, whose only network is loopback. Chromium fails every load in
  flight (`ERR_NETWORK_CHANGED`) when a host interface comes or goes, and
  testbed runs on the VM make and drop Docker networks all the time. In
  the box:
  - The filesystem is read-only except the run's work dir,
    target/storybook/compare, and `--report`, `--screenshots` and
    `--summary` where they lie outside it. The bases' builds stay
    read-only: the run outside the box makes them.
  - /run, /tmp and the other dirs that hold the host's sockets are
    empty, so a box's files (a harness script) must sit outside /tmp.
  - The snap's Chromium runs unwrapped and without its sandbox.
- Where bwrap can't make the box, the run warns and shares the host's
  network. A failed load of a build's file then fails the story in the
  plain check, and a compare captures the story again in a fresh
  browser.
- Test a change to what a compare skips or captures by the decision:
  `--list` dry runs, scope_test.py's fixture storybook
  (hack/storybook/testdata/scope) and report_test.py, each mechanism
  knocked out once to see its test fail. `--no-scope` sweeps and
  `--leaks` are for changes to how captures run or documents are reused,
  not for re-proving determinism.

## CI

- pr-test.yml's Web job runs a frozen install, then typecheck, lint and
  test:js.
- storybook.yml runs report_test.py, scope_test.py and `--list --strict`.
  It then compares `--all-modes --fail-on-error` against the push's
  previous tip or a pull request's base. Changes go to the step summary
  and the storybook-report artifact and never fail the job: visual
  changes land on purpose. A head-build story failure, an exit 2 or a
  catalog gap fails the job.
- The browser suites in crates/strims-wasm/tests (ui_smoke, ui_watch, e2e)
  stay out of CI. They need wasm builds, Firefox and ffmpeg.
  `hack/check.sh --full` runs all of them but ui_watch, whose header gives
  its command.
