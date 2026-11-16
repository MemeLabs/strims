#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""hack/console/ui.py's commands as a caller runs them (python3 ui.py
ARGS), over test_views' fixture catalog (golden_cli's, its bins given the
columns run_timeline reads) and a console server of their own (server.py
on a free port, its state file in the fixture's checkout):

  Offline   describe VIEW, describe URL and link with no server and no
            catalog; link's refusals
  Catalog   rows of a URL, as a table and as JSON; link --check passing,
            and refusing a bad cell; rows with no catalog
  NoServer  tabs, describe --tab, rows --tab and go say how to start the
            server, with no state file and with a stale one
  Server    tabs; describe --tab and rows --tab of the latest focused tab
            and of a tab by its id; a link's host the server's, then the
            latest focused tab's; a second server's own state file, its
            tabs among the first's, and the first's file left at its exit
  Go        go over a server of its own, its tabs played as the page
            plays one (Page): to a view's address, checked, and a URL;
            the tab's own address changed and its selection dropped; a
            tab in the background at its next report; a page that
            predates go, one that never takes it, a tab that isn't there

  python3 hack/simlab/tests/test_ui.py
"""

import http.client
import json
import shlex
import signal
import subprocess
import sys
import tempfile
import threading
import time
import unittest
import urllib.parse
from pathlib import Path

HERE = Path(__file__).resolve().parent
HACK = HERE.parents[1]
sys.path.insert(0, str(HACK))
sys.path.insert(0, str(HERE))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

import golden_cli  # noqa: E402
import test_views  # noqa: E402
from simlab import views  # noqa: E402

UI = HACK / "console" / "ui.py"
SERVER = HACK / "console" / "server.py"
START = "start one with python3 hack/console/server.py (after pnpm run console:build)"
RUN = ["--param", "exp=sf0001", "--param", "cell=star-5", "--param", "arm=a", "--param", "seed=2001"]
STALL_SEL = {"panel": "2", "at": {"t_s": [20, 21], "viewer": 3}}


def environment(root):
    """golden_cli's environment for a fixture at `root` (its cache, and
    its checkout's git), with no console host of the caller's."""
    env = golden_cli.environment(root)
    env.pop("SIM_CONSOLE_URL", None)
    return env


def setUpModule():
    global TMP, ROOT, EMPTY, URL
    TMP = tempfile.TemporaryDirectory(prefix="simlab-ui-")
    ROOT = Path(TMP.name) / "fixture"
    test_views.catalog_fixture(ROOT)
    # A checkout with no catalog and no server.
    EMPTY = Path(TMP.name) / "empty"
    (EMPTY / "target" / "sim-db").mkdir(parents=True)
    subprocess.run(["git", "init", "-q", str(EMPTY)], check=True)
    URL = views.link("run_timeline", {"exp": "sf0001", "cell": "star-5", "arm": "a", "seed": "2001"}, STALL_SEL,
                     base=views.BASE)


def tearDownModule():
    TMP.cleanup()


def ui(*args, root=None):
    """ui.py ARGS over the fixture at `root` (the catalog's by default):
    (exit code, stdout, stderr)."""
    p = subprocess.run([sys.executable, str(UI), *args], env=environment(root or ROOT), capture_output=True, text=True,
                       timeout=300)
    return p.returncode, p.stdout, p.stderr


class Offline(unittest.TestCase):
    def test_describe_a_view_and_an_address(self):
        code, out, err = ui("describe", "run_timeline", root=EMPTY)
        self.assertEqual((code, err), (0, ""))
        lines = out.splitlines()
        self.assertEqual(lines[0], "run_timeline: Run timeline")
        for want in ("graph: 10 panels; the time panels share the time field t_s (its cursor, and a range's band)",
                     f'  panel "{test_views.STALLS}" [2]  (time t_s, spans to t2_s)',
                     "    filter: datum.track === 'stall'", "    selectable: t_s, t2_s, viewer"):
            self.assertIn(want, lines)
        self.assertRegex(out, r"(?m)^    colours, color series: startup #[0-9a-f]{6}, after_down #")
        code, out, err = ui("describe", URL, root=EMPTY)
        self.assertEqual((code, err), (0, ""))
        lines = out.splitlines()
        for want in (f"address: {URL}",
                     "params: exp=sf0001, cell=star-5, arm=a, seed=2001, delay=1250, min_stall_ms=50",
                     f"selection: {test_views.STALLS} · t_s 20–21 · viewer 3",
                     "  its rows: datum.track === 'stall' and t_s to t2_s overlaps 20 to 21 and viewer = 3",
                     f"  python3 hack/console/ui.py rows {shlex.quote(URL)}",
                     "  python3 hack/sim-db/simdb.py view run_timeline --param exp=sf0001 --param cell=star-5 "
                     "--param arm=a --param seed=2001 --param delay=1250 --param min_stall_ms=50",
                     "graph: 10 panels; the time panels share the time field t_s (its cursor, and a range's band)"):
            self.assertIn(want, lines)
        code, out, err = ui("describe", root=EMPTY)
        self.assertEqual((code, err), (0, ""))
        self.assertRegex(out, r"(?m)^\| run_timeline +\| Run timeline +\| 10 +\|$")

    def test_link_writes_an_address(self):
        code, out, err = ui("link", "run_timeline", *RUN, "--sel", json.dumps(STALL_SEL), root=EMPTY)
        self.assertEqual((code, out, err), (0, URL + "\n", ""))
        code, out, err = ui("link", "workbench", "--sql", "SELECT t, v FROM x WHERE exp_id = $exp", "--param",
                            "exp=e1", "--chart", "line,x=t,y=v,logy", "--base", "http://box:9000/", root=EMPTY)
        self.assertEqual((code, err), (0, ""))
        self.assertEqual(out, "http://box:9000/devtools.html#/console/workbench?sql=SELECT+t%2C+v+FROM+x+WHERE+exp_id"
                              "+%3D+%24exp&exp=e1&mark=line&x=t&y=v&logy=1\n")
        self.assertEqual(views.parse(out.strip())[1], {"sql": "SELECT t, v FROM x WHERE exp_id = $exp", "exp": "e1",
                                                       "mark": "line", "x": "t", "y": "v", "logy": "1"})

    def test_what_link_refuses(self):
        for args, message in [
            (["run_timeline", *RUN, "--param", "arm2=b"], "link: run_timeline takes no arm2: its params are exp, "),
            (["run_timeline", "--sel", "{"], "link: --sel isn't JSON: {"),
            (["run_timeline", "--sel", '{"at": {"lane": "x"}}'], "link: run_timeline encodes no column 'lane'"),
            (["tree", "--sql", "SELECT 1"], "link: --sql, --view and --chart are the workbench's"),
            (["nope"], "link: no view 'nope': collapse_diff, "),
        ]:
            with self.subTest(args=args):
                code, out, err = ui("link", *args, root=EMPTY)
                self.assertEqual((code, out), (1, ""))
                self.assertTrue(err.startswith(message), err)
        code, _, err = ui("link", "workbench", "--chart", "pie", root=EMPTY)
        self.assertEqual(code, 2)
        self.assertIn("'pie': a mark (line, point, bar, area, rect, tick), logy", err)


class Catalog(unittest.TestCase):
    def test_rows_of_an_address(self):
        code, out, err = ui("rows", URL)
        self.assertEqual(code, 0, err)
        self.assertEqual(out.splitlines()[3], "| stall | 18.0 | 22.75 | NULL   | 3      | 4750.0 | NULL   |")
        self.assertEqual(len([line for line in out.splitlines() if line.startswith("| ")]), 2)
        self.assertEqual(err, "rows: run_timeline exp=sf0001, cell=star-5, arm=a, seed=2001, delay=1250, "
                              f"min_stall_ms=50; selection {test_views.STALLS} · t_s 20–21 · viewer 3: "
                              "datum.track === 'stall' and t_s to t2_s overlaps 20 to 21 and viewer = 3\n")
        code, out, err = ui("rows", URL, "--json")
        self.assertEqual(code, 0, err)
        self.assertEqual(json.loads(out), [{"track": "stall", "t_s": 18.0, "t2_s": 22.75, "series": None, "viewer": 3,
                                            "value": 4750.0, "detail": None}])
        code, out, err = ui("rows", URL, root=EMPTY)
        self.assertEqual((code, out), (1, ""))
        self.assertRegex(err, r"^no catalog at .*: run python3 hack/sim-db/simdb.py index\n$")

    def test_link_check(self):
        code, out, err = ui("link", "run_timeline", *RUN, "--sel", json.dumps(STALL_SEL), "--check")
        self.assertEqual((code, out, err), (0, URL + "\n", ""))
        code, out, err = ui("link", "run_timeline", *RUN, "--param", "cell=star-9", "--check")
        self.assertEqual((code, out, err), (1, "", "link: sf0001 has no cell 'star-9': its cells are halve-5, "
                                                   "star-5\n"))
        code, out, err = ui("link", "collapse_grid", "--param", "exp=sf0001", "--sel",
                            '{"at": {"cell": "star-5", "seed": 2009}}', "--check")
        self.assertEqual((code, out, err), (1, "", "link: the selection matches no row of collapse_grid\n"))


def no_server(test, root):
    for args in (["tabs"], ["describe", "--tab"], ["rows", "--tab"], ["describe", "--tab", "t1"], ["go", "overhead"]):
        with test.subTest(args=args):
            code, out, err = ui(*args, root=root)
            test.assertEqual((code, out), (1, ""))
            test.assertEqual(err, f"{args[0]}: no console server is running: {START}\n")


class NoServer(unittest.TestCase):
    def test_without_a_server_each_command_says_how_to_start_one(self):
        no_server(self, EMPTY)

    def test_a_state_file_its_server_left_is_none(self):
        dead = subprocess.Popen(["true"])
        dead.wait()
        root = Path(TMP.name) / "stale"
        (root / "target" / "console" / "servers").mkdir(parents=True)
        subprocess.run(["git", "init", "-q", str(root)], check=True)
        (root / "target" / "console" / "servers" / "1.json").write_text(json.dumps(
            {"pid": dead.pid, "port": 1, "urls": ["http://127.0.0.1:1/"]}))
        no_server(self, root)
        code, out, _ = ui("link", "overhead", root=root)
        self.assertEqual((code, out), (0, f"{views.BASE}{views.PAGE}overhead\n"))


def start_server():
    """A server.py over the fixture, on a free port: (its process, its
    state file)."""
    states = ROOT / "target" / "console" / "servers"
    before = set(states.glob("*.json"))
    proc = subprocess.Popen([sys.executable, str(SERVER), "--port", "0", "--host", "127.0.0.1", "--no-index"],
                            env=environment(ROOT), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    end = time.time() + 60
    while time.time() < end and proc.poll() is None:
        new = [f for f in states.glob("*.json") if f not in before]
        if new:
            return proc, new[0]
        time.sleep(0.05)
    proc.kill()
    raise RuntimeError("server.py wrote no state file")


def stop_server(proc):
    proc.send_signal(signal.SIGTERM)
    proc.wait(timeout=60)


class Page:
    """A console tab as api.ts plays one, at server `port`: it reports its
    address every `every` s (with `took`, but for a page that predates go,
    took None), holds its goto stream while in view, and goes to each
    address it's sent (on its own host), then reports with its seq; in the
    background it takes one from its report's answer. One that doesn't
    `take` (a page stuck) holds no stream and takes none."""

    def __init__(self, test, port, id, url, visible=True, took=0, takes=True, every=0.5):
        self.port, self.id, self.url, self.visible, self.took = port, id, url, visible, took
        self.takes = takes
        self.lock = threading.Lock()
        self.report()
        stop = threading.Event()
        test.addCleanup(stop.set)

        def reports():
            while not stop.wait(every):
                self.report()
        threading.Thread(target=reports, daemon=True).start()
        if visible and takes:
            c = http.client.HTTPConnection("127.0.0.1", port, timeout=120)
            c.request("GET", f"/console/tabs/{id}/goto?took={took or 0}")
            r = c.getresponse()
            test.assertEqual(r.status, 200)
            threading.Thread(target=self.follow, args=(r,), daemon=True).start()
            test.addCleanup(c.close)

    def follow(self, r):
        try:
            for line in r.fp:
                got = json.loads(line)
                if got.get("type") == "goto":
                    self.take(got)
        except (OSError, ValueError):
            pass

    def take(self, goto):
        with self.lock:
            if goto["seq"] <= self.took:
                return
            self.url = self.url.partition("#")[0] + "#" + urllib.parse.urlsplit(goto["url"]).fragment
            self.took = goto["seq"]
        self.report()

    def report(self):
        with self.lock:
            body = {"url": self.url, "focused": True, "visible": self.visible}
            if self.took is not None:
                body["took"] = self.took
        c = http.client.HTTPConnection("127.0.0.1", self.port, timeout=30)
        try:
            c.request("PUT", f"/console/tabs/{self.id}", json.dumps(body), {"Content-Type": "application/json"})
            answer = json.loads(c.getresponse().read())
        except (OSError, ValueError):
            return
        finally:
            c.close()
        if answer.get("goto") and self.took is not None and self.takes:
            self.take(answer["goto"])


class Go(unittest.TestCase):
    """go over a server of its own (its tabs gone with it), the tabs Page's."""
    HOST = "http://blinky.local:8086"

    @classmethod
    def setUpClass(cls):
        cls.proc, cls.state = start_server()
        cls.port = json.loads(cls.state.read_text())["port"]

    @classmethod
    def tearDownClass(cls):
        stop_server(cls.proc)

    def test_a_tab_goes_to_an_address_and_changes_its_own(self):
        grid = views.link("collapse_grid", {"exp": "sf0001"}, base=self.HOST)
        page = Page(self, self.port, "g1", grid)
        want = URL.replace(views.BASE, self.HOST)
        code, out, err = ui("go", "run_timeline", *RUN, "--sel", json.dumps(STALL_SEL), "--tab", "g1", "--check")
        self.assertEqual((code, out, err), (0, want + "\n", f"go: tab g1 was at {grid}\n"))
        self.assertEqual(page.url, want)
        # With no VIEW, the tab's own address changes: a param set, the selection replaced, then dropped.
        code, out, err = ui("go", "--tab", "g1", "--param", "seed=2002", "--sel", '{"panel": "2", "at": {"t_s": [30, 31]}}')
        self.assertEqual((code, err), (0, f"go: tab g1 was at {want}\n"))
        name, params, sel = views.parse(out)
        self.assertEqual((name, params["cell"], params["seed"], sel),
                         ("run_timeline", "star-5", "2002", {"panel": test_views.STALLS, "at": {"t_s": [30, 31]}}))
        self.assertEqual(page.url, out.strip())
        code, out, err = ui("go", "--tab", "g1", "--sel", "null")
        self.assertEqual(code, 0, err)
        self.assertEqual(views.parse(out)[1:], (params, None))
        self.assertEqual(page.url, out.strip())
        # A URL as given; the address checked first, a bad one going nowhere.
        self.assertEqual(ui("go", grid, "--tab", "g1")[:2], (0, grid + "\n"))
        code, out, err = ui("go", "run_timeline", *RUN, "--param", "cell=star-9", "--check", "--tab", "g1")
        self.assertEqual((code, out, err), (1, "", "go: sf0001 has no cell 'star-9': its cells are halve-5, star-5\n"))
        self.assertEqual(page.url, grid)
        self.assertEqual(ui("go", "--tab", "g1"), (1, "", "go: give a VIEW or a URL, or --param or --sel to change "
                                                           "the tab's own address\n"))
        code, out, err = ui("go", "overhead", "--tab", "nope")
        self.assertEqual((code, out), (1, ""))
        self.assertTrue(err.startswith("go: no tab 'nope': the tabs are "), err)

    def test_a_tab_in_the_background_and_ones_that_cant_go(self):
        grid = views.link("collapse_grid", {"exp": "sf0001"}, base=self.HOST)
        overhead = views.link("overhead", {"exp": "sf0001"}, base=self.HOST)
        page = Page(self, self.port, "bg", grid, visible=False)
        code, out, err = ui("go", "overhead", "--param", "exp=sf0001", "--tab", "bg")
        self.assertEqual((code, out, err), (0, overhead + "\n", "go: tab bg is in the background: it goes there at "
                                                                 "its next report (up to a minute) or when it's shown\n"
                                                                 f"go: tab bg was at {grid}\n"))
        self.assertEqual(page.url, overhead)
        Page(self, self.port, "old", grid, took=None)
        self.assertEqual(ui("go", "overhead", "--tab", "old"), (1, "", "go: tab old's page predates goto: reload it on "
                                                                       "a current build (pnpm run console:build)\n"))
        Page(self, self.port, "frozen", grid, takes=False)
        self.assertEqual(ui("go", "overhead", "--tab", "frozen", "--wait", "1"),
                         (1, "", "go: tab frozen didn't go there in 1 s, so the address is withdrawn: reload it on a "
                                 "current build (pnpm run console:build)\n"))
        self.assertEqual(ui("go", "overhead", "--wait", "0")[2],
                         "go: --wait is seconds, more than 0 and at most 120\n")


class Server(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proc, cls.state = start_server()
        cls.port = json.loads(cls.state.read_text())["port"]

    @classmethod
    def tearDownClass(cls):
        stop_server(cls.proc)

    def report(self, id, url, focused, port=None):
        c = http.client.HTTPConnection("127.0.0.1", port or self.port, timeout=30)
        try:
            c.request("PUT", f"/console/tabs/{id}", json.dumps({"url": url, "focused": focused, "visible": True}),
                      {"Content-Type": "application/json"})
            r = c.getresponse()
            r.read()
            self.assertEqual(r.status, 200)
        finally:
            c.close()

    def test_every_running_servers_tabs(self):
        second, state = start_server()
        try:
            port = json.loads(state.read_text())["port"]
            self.assertEqual(state.name, f"{port}.json")
            # With no tab, a link is on the first server's address, in port order.
            first = min(port, self.port)
            self.assertEqual(ui("link", "overhead")[1], f"http://127.0.0.1:{first}{views.PAGE}overhead\n")
            url = views.link("overhead", {"exp": "sf0001"}, base=f"http://blinky.local:{port}")
            self.report("s2", url, True, port=port)
            code, out, err = ui("tabs")
            self.assertEqual((code, err), (0, ""))
            rows = [[c.strip() for c in line.split("|")[1:-1]] for line in out.splitlines() if line.startswith("| ")]
            self.assertEqual([r[:3] + r[4:] for r in rows[1:]], [["s2", "yes", "yes", "overhead", url]])
            self.assertEqual(ui("link", "overhead", "--param", "exp=sf0001")[1], url + "\n")
        finally:
            stop_server(second)
        self.assertFalse(state.exists())
        self.assertTrue(self.state.exists())
        self.assertEqual(ui("tabs"), (0, "no console tab has reported its address: open the console in a browser\n",
                                      ""))

    def test_the_tabs_and_their_addresses(self):
        here = f"http://127.0.0.1:{self.port}"
        code, out, err = ui("tabs")
        self.assertEqual((code, out, err), (0, "no console tab has reported its address: open the console in a "
                                               "browser\n", ""))
        code, _, err = ui("describe", "--tab")
        self.assertEqual((code, err), (1, f"describe: no console tab has reported its address: open the console in a "
                                          f"browser ({views.BASE}/ or the server's address)\n"))
        # With no tab, a link is on the server's address.
        self.assertEqual(ui("link", "overhead", "--param", "exp=sf0001")[1],
                         f"{here}{views.PAGE}overhead?exp=sf0001\n")
        grid = views.link("collapse_grid", {"exp": "sf0001"}, {"at": {"cell": "star-5", "seed": 2001}},
                          base="http://blinky.local:8086")
        tab_url = URL.replace(views.BASE, "http://blinky.local:8086")
        self.report("t1", grid, True)
        self.report("t2", tab_url, True)
        ask = "http://blinky.local:8086/devtools.html#/console/ask"
        self.report("t3", ask, False)
        code, out, err = ui("tabs")
        self.assertEqual((code, err), (0, ""))
        rows = [[c.strip() for c in line.split("|")[1:-1]] for line in out.splitlines() if line.startswith("| ")]
        self.assertEqual(rows[0], ["tab", "focused", "visible", "seen (s ago)", "view", "address"])
        # The latest focused first; what each said, but how long ago.
        self.assertEqual([r[:3] + r[4:] for r in rows[1:]], [["t2", "yes", "yes", "run_timeline", tab_url],
                                                              ["t1", "yes", "yes", "collapse_grid", grid],
                                                              ["t3", "no", "yes", "?", ask]])
        self.assertTrue(all(float(r[3]) < 60 for r in rows[1:]))
        code, out, err = ui("describe", "--tab")
        self.assertEqual((code, err), (0, ""))
        self.assertIn(f"address: {tab_url}", out)
        self.assertRegex(out, r"(?m)^tab t2: focused, visible, heard from [0-9.]+ s ago$")
        self.assertIn(f"selection: {test_views.STALLS} · t_s 20–21 · viewer 3", out)
        code, out, err = ui("describe", "--tab", "t1")
        self.assertEqual((code, err), (0, ""))
        self.assertTrue(out.startswith("collapse_grid: Collapse grid\n"), out)
        self.assertIn("selection: cell \"star-5\" · seed 2001", out)
        code, out, err = ui("rows", "--tab", "--json")
        self.assertEqual(code, 0, err)
        self.assertEqual([r["viewer"] for r in json.loads(out)], [3])
        code, out, err = ui("rows", "--tab", "t1", "--json")
        self.assertEqual(code, 0, err)
        self.assertEqual(sorted(r["arm"] for r in json.loads(out)), ["a", "off"])
        code, out, err = ui("describe", "--tab", "nope")
        self.assertEqual((code, out, err), (1, "", "describe: no tab 'nope': the tabs are t2, t1, t3\n"))
        code, out, err = ui("describe", "--tab", "t3")
        self.assertEqual((code, out), (1, ""))
        self.assertIn("is the console's ask page, which shows no view", err)
        # A link is on the host the latest focused tab reached.
        self.assertEqual(ui("link", "overhead", "--param", "exp=sf0001")[1],
                         f"http://blinky.local:8086{views.PAGE}overhead?exp=sf0001\n")


if __name__ == "__main__":
    unittest.main()
