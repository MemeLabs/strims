---
name: sim-console
description: "The sim console's addresses: reading what the user has selected on a console graph (a clicked value or a brushed time range, from their open tab or a pasted console URL), and giving them links that open a view with a value or time range highlighted. Use when the user refers to their selection, chart, console tab or a console URL, or asks to be shown a result or given a link to one. For computing or checking results, use sim-lab."
---

# The sim console's addresses

The console (`hack/console/server.py`) draws the saved views' graphs.
Its address holds the view, its params and what is highlighted:
`/devtools.html#/console/VIEW?exp=…&cell=…&sel={"panel": …, "at": {…}}`.
A click selects a mark's values, and a drag brushes a range of a time
panel's time field. Each open tab reports its address to the server.
`hack/console/ui.py` reads and writes these addresses. Run it as
`python3 hack/console/ui.py` from any checkout or worktree.

## Start here

    python3 hack/console/ui.py describe --tab        # the user's latest focused tab: view, params, selection, graph
    python3 hack/console/ui.py describe 'URL'        # the same for a pasted console URL
    python3 hack/console/ui.py describe VIEW         # a view's graph: panels, channels, time field, colours
    python3 hack/console/ui.py tabs                  # every open tab and its address

`describe` gives the selection in words, the panel it's on, and the
commands that give its rows. A panel's id is its title. The colours
name the vocabulary's phases, episodes and reasons, so "the pink band"
maps to a name.

## Reading a selection

    python3 hack/console/ui.py rows --tab [--json]   # the rows behind the selection
    python3 hack/console/ui.py rows 'URL' [--json]

From there, the analysis is sim-lab's: `simdb.py lake` around that run
and time, under its rules. Analyse rows, never a rendering of the
chart.

## Pointing the user at a result

    python3 hack/console/ui.py link VIEW --param exp=… --param cell=… --sel '{"panel": "ID", "at": {"t_s": [38, 46]}}' --check
    python3 hack/console/ui.py link workbench --sql 'SELECT …' --chart line,x=t_s,y=lag_ms,color=seed --check

- `--sel` takes a value per field the panel encodes (`"viewer": 7`), or
  a range `[LO, HI]` on its time field. `describe VIEW` lists each
  panel's selectable columns and time field.
- `--check` confirms the experiment, cell, arms and seed ran and that
  the selection matches a row. Give links with it.
- Never write a console URL by hand. `SIM_CONSOLE_URL` sets the host.
  Otherwise the host is the one the user's latest tab reached.

## The server

`--tab` and `tabs` need the console server. When it isn't running, tell
the user to start it rather than starting it yourself:
`python3 hack/console/server.py`, after `npm run console:build`, then
http://127.0.0.1:8086/. The page's Ask panel answers questions there.
