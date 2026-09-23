"""Installed public-API campaign controls with synthetic peers, never native bodies."""

from contextlib import contextmanager
import copy
import importlib.util
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import threading
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import zipfile

from crebain_ncp_sensors import BodySession, codec as c, new_binding
from ncp_local import modular_owner as o, wire as framing
from prisoma_agent_bridge.crebain import SensorExperiment

from synthetic_sensors import SyntheticSensors


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "m1_campaign.py"
SPEC = importlib.util.spec_from_file_location("m1_campaign", SCRIPT)
m = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)
WORKLOAD = Path(__file__).resolve().parent / "fixtures" / "m1.workload.v1.json"


@contextmanager
def channel(selected_binding, application):
    owner = o.Owner(
        selected_binding, application, tuple(sorted(c.SENSOR_DIGESTS.values()))
    )
    first, second = socket.socketpair()
    stream, peer = (
        first.makefile("rwb", buffering=0),
        second.makefile("rwb", buffering=0),
    )
    errors = []

    def serve():
        try:
            deadline = time.monotonic() + 60
            while (
                request := framing.read_local_frame(peer, deadline=deadline)
            ) is not None:
                framing.write_local_frame(
                    peer, bytes(owner.process(request)), deadline=deadline
                )
        except (BrokenPipeError, ConnectionResetError):
            pass
        except BaseException as error:
            errors.append(error)
        finally:
            peer.close()
            second.close()

    thread = threading.Thread(target=serve)
    thread.start()
    try:
        yield stream
    finally:
        try:
            first.shutdown(socket.SHUT_RDWR)
        except OSError:
            pass
        stream.close()
        first.close()
        thread.join(5)
        if thread.is_alive():
            raise AssertionError("synthetic peer failed to retire")
        if errors:
            raise errors[0]


class CampaignTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name).resolve()
        self.prepare, self.targets, self.expected = m.workload(
            m.file_identity(WORKLOAD)
        )
        self.freeze_digest = "d" * 64
        artifact = m.file_identity(SCRIPT)
        self.freeze = {
            "schema": "prisoma.m1-freeze.v1",
            "campaign_id": "synthetic-controls",
            "workload": m.file_identity(WORKLOAD),
            "source": {"commit": "1" * 40},
            "tools": {"observer_binary": artifact},
            "runtime": {"source_identity": "a" * 64},
            "environments": {
                "body": {"prefix": "body"},
                "canonical": {"prefix": "canonical"},
            },
            "output_root": str(self.root),
            "cases": [
                {"case_id": "body", "arm": "body", "fault": "none"},
                {"case_id": "canonical", "arm": "canonical", "fault": "none"},
            ],
        }

    def run_case(self, arm):
        output = m.Output(self.root / arm)
        case = m.selected_case(self.freeze, arm)
        selected_binding, receipt, binding_digest = m.bind_case(
            self.freeze, self.freeze_digest, case, output
        )
        application = SyntheticSensors()
        state = {
            "prepared": False,
            "attempted_tick": None,
            "validated_ticks": 0,
            "exported_ticks": 0,
            "explicit_finish": False,
            "capture_finalized": None,
            "canonical_finalized": None,
            "session_result": None,
            "selection_rechecked": True,
        }
        checkpoints = []

        @contextmanager
        def selected_owner(stream, recorder=None):
            if arm == "canonical":
                session = SensorExperiment(
                    output.path / "run.jsonl",
                    output.path / "capture.ncp",
                    stream,
                    stream,
                    selected_binding,
                    self.prepare,
                    deadline=time.monotonic() + 60,
                )
            else:
                session = BodySession(
                    stream,
                    stream,
                    selected_binding,
                    self.prepare,
                    deadline=time.monotonic() + 60,
                    exchange=recorder.exchange,
                )
            # A synthetic owner supplies the independently stated retirement
            # fixture. These tests never claim operating-system retirement.
            with session:
                yield session
            if arm == "canonical":
                session._session.process_exit = {
                    "returncode": 0,
                    "cleanup_confirmed": True,
                }
            else:
                session.process_exit = {"returncode": 0, "cleanup_confirmed": True}

        with channel(selected_binding, application) as stream:
            if arm == "canonical":
                m.execute_session(
                    selected_owner(stream),
                    canonical_arm=True,
                    prepare=self.prepare,
                    targets=self.targets,
                    output=output,
                    notify=checkpoints.append,
                    state=state,
                )
            else:
                with output.open("wire.bin") as wire:
                    recorder = m.WireRecorder(output, wire, self.expected["exchanges"])
                    m.execute_session(
                        selected_owner(stream, recorder),
                        canonical_arm=False,
                        prepare=self.prepare,
                        targets=self.targets,
                        output=output,
                        notify=checkpoints.append,
                        state=state,
                    )
                self.assertEqual(recorder.count, 476)
        self.assertEqual(checkpoints, ["prepared", "tick1"])
        result = m.terminal_result(output, receipt, binding_digest, state)
        return output, result

    def rehash(self, output, name):
        result = m.parse(m.read(output.path / "result.json"))
        raw = m.read(output.path / name)
        result["artifacts"][name] = {
            "path": name,
            "bytes": len(raw),
            "sha256": m.digest(raw),
        }
        (output.path / "result.json").write_bytes(m.canonical(result) + b"\n")

    def test_exact_original_workload_accounting(self):
        self.assertEqual(
            m.file_identity(WORKLOAD)["sha256"],
            "f4b8cad4a9daf1f5b95abb9b03409040cd8caa67cf2e14c074a17dcc4a77ecae",
        )
        self.assertEqual(
            self.expected,
            {
                "ticks": 24,
                "payloads": 44,
                "raw_bytes": 4326400,
                "pressure_samples": 3200,
                "chunks": 168,
                "exchanges": 476,
                "max_call_exchanges": 36,
                "canonical_calls": 26,
                "canonical_events": 82,
            },
        )
        self.assertEqual(list(self.targets), [1, 13])
        self.assertEqual(self.targets[13].pitch_rad, 0.03)

    def test_both_public_arms_reconstruct_all_original_bytes(self):
        body, _ = self.run_case("body")
        canonical, _ = self.run_case("canonical")
        with (
            patch(
                "subprocess.Popen",
                side_effect=AssertionError("replay launched a child"),
            ),
            patch(
                "socket.socket", side_effect=AssertionError("replay opened a socket")
            ),
        ):
            left = m.verify_arm(
                self.freeze, self.freeze_digest, "body", body.path, require_owner=False
            )
            right = m.verify_arm(
                self.freeze,
                self.freeze_digest,
                "canonical",
                canonical.path,
                require_owner=False,
            )
        self.assertEqual(left["semantic"], right["semantic"])
        self.assertEqual(
            sum(
                len(row["payload"])
                for step in left["semantic"]
                for row in step["payloads"]
            ),
            4326400,
        )
        self.assertNotEqual(left["binding"]["binding"], right["binding"]["binding"])
        self.assertEqual(right["canonical"]["canonical"]["events"], 82)

    def test_matching_but_wrong_workload_fails_original_reconstruction(self):
        output, _ = self.run_case("body")
        changed = copy.deepcopy(self.targets)
        changed[13] = c.decode("SetTarget", {**c.raw(changed[13]), "pitch_rad": 0.01})
        rows = [
            m.parse(line) for line in m.read(output.path / "steps.jsonl").splitlines()
        ]
        binding = m._binding_type(
            m.parse(m.read(output.path / "binding.json"))["binding"]
        )
        with self.assertRaisesRegex(Exception, "sensor session stopped") as stopped:
            m.replay_pairs(
                m._wire_pairs(output.path / "wire.bin"),
                binding,
                self.prepare,
                changed,
                rows,
                output.path,
                self.expected,
                "a" * 64,
            )
        self.assertIsInstance(stopped.exception.__cause__, m.CampaignError)
        self.assertEqual(stopped.exception.__cause__.code, "original_request")

    def test_copied_command_and_terminal_controls_reach_deep_verifiers(self):
        output, _ = self.run_case("canonical")
        result = m.copied_negatives(output.path / "run.jsonl", self.root / "copies")
        self.assertEqual(
            result["controls"],
            {
                "copied-command": "original_request_rejected",
                "capture-terminal": "terminal_record_digest_rejected",
            },
        )
        m.verify_arm(
            self.freeze,
            self.freeze_digest,
            "canonical",
            output.path,
            require_owner=False,
        )

    def test_exported_payload_change_rejects_even_after_outer_rehash(self):
        output, _ = self.run_case("body")
        name = "payload-00000.bin"
        raw = m.read(output.path / name)
        (output.path / name).write_bytes(bytes([raw[0] ^ 1]) + raw[1:])
        self.rehash(output, name)
        with self.assertRaises(m.CampaignError):
            m.verify_arm(
                self.freeze,
                self.freeze_digest,
                "body",
                output.path,
                require_owner=False,
            )

    def test_stale_freeze_case_generation_and_environment_join_reject(self):
        output, _ = self.run_case("body")
        path = output.path / "binding.json"
        original = m.read(path)
        changes = {
            "stale-freeze": lambda row: row.update(freeze_sha256="e" * 64),
            "wrong-case": lambda row: row["case"].update(case_id="canonical"),
            "mixed-wheel": lambda row: row["environment"].update(prefix="canonical"),
            "wrong-runtime": lambda row: row["runtime"].update(
                source_identity="b" * 64
            ),
            "wrong-source": lambda row: row["source"].update(commit="2" * 40),
            "wrong-directory": lambda row: row["output_owner"].update(inode=0),
        }
        for label, change in changes.items():
            with self.subTest(label=label):
                value = m.parse(original)
                change(value)
                path.write_bytes(m.canonical(value) + b"\n")
                self.rehash(output, "binding.json")
                with self.assertRaisesRegex(m.CampaignError, "case_binding"):
                    m.verify_arm(
                        self.freeze,
                        self.freeze_digest,
                        "body",
                        output.path,
                        require_owner=False,
                    )
        path.write_bytes(original)
        self.rehash(output, "binding.json")
        m.verify_arm(
            self.freeze, self.freeze_digest, "body", output.path, require_owner=False
        )

    def test_run_identity_change_cannot_be_repaired_by_result_rehash(self):
        output, _ = self.run_case("body")
        path = output.path / "binding.json"
        row = m.parse(m.read(path))
        row["binding"]["generation"] = new_binding().generation
        raw = m.canonical(row) + b"\n"
        path.write_bytes(raw)
        self.rehash(output, "binding.json")
        result = m.parse(m.read(output.path / "result.json"))
        result["binding_sha256"] = m.digest(raw)
        (output.path / "result.json").write_bytes(m.canonical(result) + b"\n")
        with self.assertRaises(Exception):
            m.verify_arm(
                self.freeze,
                self.freeze_digest,
                "body",
                output.path,
                require_owner=False,
            )

    def test_fresh_output_rejects_reuse_and_symlink_input(self):
        m.Output(self.root / "case")
        with self.assertRaises(FileExistsError):
            m.Output(self.root / "case")
        linked = self.root / "linked.json"
        linked.symlink_to(WORKLOAD)
        with self.assertRaisesRegex(m.CampaignError, "direct_path"):
            m.file_identity(linked)
        self.assertEqual(m.file_identity(WORKLOAD)["bytes"], 3444)

    def test_missing_false_or_mixed_owner_receipt_cannot_qualify(self):
        output, result = self.run_case("body")
        with self.assertRaises(FileNotFoundError):
            m.verify_arm(self.freeze, self.freeze_digest, "body", output.path)
        # Structured fixture, not a claim about operating-system observation.
        observation = {
            "schema": "prisoma.m1-owner-observation.v1",
            "freeze_sha256": self.freeze_digest,
            "case_id": "body",
            "observation": {
                "schema": "local.observed-owned-tree.v1",
                "observed_identity_count": 1,
                "observed_identities_retired": True,
                "remaining_births": [],
                "events": [
                    {
                        "kind": "observed",
                        "identity": {
                            "pid": 100,
                            "ppid": 99,
                            "uid": os.getuid(),
                            "status": 2,
                            "start_seconds": 1,
                            "start_microseconds": 0,
                        },
                    },
                    {"kind": "identity_absent", "birth": [100, 1, 0]},
                ],
                "hostile_process_containment": False,
                "signals_sent_by_observer": 0,
                "independent_application_receipt": False,
            },
            "supervisor_events": [
                {
                    "kind": "checkpoint",
                    "stage": stage,
                    "binding_sha256": result["binding_sha256"],
                }
                for stage in ("prepared", "tick1")
            ],
        }
        evidence = output.json("observer-events.json", observation)
        owner = {
            "schema": "prisoma.m1-owner.v1",
            "freeze_sha256": self.freeze_digest,
            "case_id": "body",
            "binding_sha256": result["binding_sha256"],
            "result_sha256": m.digest(m.read(output.path / "result.json")),
            "worker_returncode": 0,
            "observed_births": [[100, 1, 0]],
            "retired_births": [[100, 1, 0]],
            "emergency_cleanup": False,
            "observer_events": evidence,
            "observer_sha256": self.freeze["tools"]["observer_binary"]["sha256"],
        }
        path = output.path / "owner.json"
        path.write_bytes(m.canonical(owner))
        m.verify_arm(self.freeze, self.freeze_digest, "body", output.path)
        for key, value in [
            ("retired_births", []),
            ("emergency_cleanup", True),
            ("worker_returncode", False),
            ("result_sha256", "0" * 64),
            ("freeze_sha256", "0" * 64),
            ("observed_births", [[100, 1, 0], [100, 1, 0]]),
        ]:
            with self.subTest(key=key):
                path.write_bytes(m.canonical({**owner, key: value}))
                with self.assertRaises(m.CampaignError):
                    m.verify_arm(self.freeze, self.freeze_digest, "body", output.path)
        changes = [
            lambda row: row.update(freeze_sha256="0" * 64),
            lambda row: row.update(case_id="canonical"),
            lambda row: row["observation"].update(observed_identity_count=2),
            lambda row: row["observation"].update(observed_identities_retired=False),
            lambda row: row["observation"].update(remaining_births=[[100, 1, 0]]),
            lambda row: row["observation"].update(signals_sent_by_observer=False),
            lambda row: row["observation"]["events"].reverse(),
            lambda row: row["observation"]["events"].append(
                row["observation"]["events"][0]
            ),
            lambda row: row["observation"]["events"].append(
                row["observation"]["events"][1]
            ),
            lambda row: row["observation"].update(
                events=row["observation"]["events"] * 257
            ),
            lambda row: row["supervisor_events"].reverse(),
            lambda row: row["supervisor_events"][0].update(binding_sha256="0" * 64),
        ]
        for index, change in enumerate(changes):
            with self.subTest(observation_change=index):
                changed = copy.deepcopy(observation)
                change(changed)
                raw = m.canonical(changed)
                (output.path / evidence["path"]).write_bytes(raw)
                changed_owner = {
                    **owner,
                    "observer_events": {
                        "path": evidence["path"],
                        "bytes": len(raw),
                        "sha256": m.digest(raw),
                    },
                }
                path.write_bytes(m.canonical(changed_owner))
                with self.assertRaises(m.CampaignError):
                    m.verify_owner(
                        self.freeze,
                        self.freeze_digest,
                        self.freeze["cases"][0],
                        result["binding_sha256"],
                        owner["result_sha256"],
                        output.path,
                    )
        raw = b"synthetic observation only\n"
        (output.path / evidence["path"]).write_bytes(raw)
        path.write_bytes(
            m.canonical(
                {
                    **owner,
                    "observer_events": {
                        "path": evidence["path"],
                        "bytes": len(raw),
                        "sha256": m.digest(raw),
                    },
                }
            )
        )
        with self.assertRaises(m.CampaignError):
            m.verify_owner(
                self.freeze,
                self.freeze_digest,
                self.freeze["cases"][0],
                result["binding_sha256"],
                owner["result_sha256"],
                output.path,
            )

    def worker_cut(self, mode, *, reporting_failure=False):
        case_id = mode + ("-reporting-failure" if reporting_failure else "")
        freeze = copy.deepcopy(self.freeze)
        freeze["limits"] = {"session_seconds": 30, "checkpoint_seconds": 5}
        freeze["cases"].append(
            {"case_id": case_id, "arm": "canonical", "fault": "renderer_loss"}
        )
        primary = BaseExceptionGroup(
            "original", [RuntimeError("first"), KeyboardInterrupt("cancelled")]
        )
        runtime = object()

        @contextmanager
        def owner(_runtime, prepare, log, capture, *, binding, timeout_s):
            if mode == "preyield":
                raise primary
            application = SyntheticSensors()
            if mode == "renderer":
                application.fail_tick = 2
            with channel(binding, application) as stream:
                session = SensorExperiment(
                    log,
                    capture,
                    stream,
                    stream,
                    binding,
                    prepare,
                    deadline=time.monotonic() + timeout_s,
                )
                try:
                    with session:
                        yield session
                finally:
                    session._session.process_exit = {
                        "returncode": 1,
                        "cleanup_confirmed": False,
                    }

        def checkpoint(_binding, stage, *args):
            if stage == mode:
                raise primary

        actual_write = m.Output.write

        def write(output, name, raw):
            if reporting_failure and name == "diagnostics.bin":
                raise OSError("diagnostic write cut")
            return actual_write(output, name, raw)

        with (
            patch.object(m, "load_freeze", return_value=(freeze, self.freeze_digest)),
            patch.object(m, "environment_selection"),
            patch.object(m, "runtime_selection", return_value=runtime),
            patch.object(m, "checkpoint", side_effect=checkpoint),
            patch.object(m.Output, "write", write),
            patch("prisoma_agent_bridge.crebain.owned_sensor_experiment", owner),
        ):
            with self.assertRaises(BaseException) as caught:
                m.worker(self.root / "freeze.json", case_id, self.root / case_id, 3, 4)
        if mode != "renderer":
            self.assertIs(caught.exception, primary)
        return freeze, case_id, self.root / case_id

    def test_actual_worker_failure_writer_preserves_known_prefix_and_original(self):
        for mode, prepared, attempted, validated in [
            ("preyield", False, None, 0),
            ("prepared", True, None, 0),
            ("tick1", True, 1, 1),
            ("renderer", True, 2, 1),
        ]:
            with self.subTest(mode=mode):
                _, _, output = self.worker_cut(mode)
                result = m.parse(m.read(output / "result.json"))
                self.assertIs(result["healthy"], False)
                self.assertEqual(result["completion"]["prepared"], prepared)
                self.assertEqual(result["completion"]["attempted_tick"], attempted)
                self.assertEqual(result["completion"]["validated_ticks"], validated)
                self.assertEqual(result["completion"]["exported_ticks"], validated)
                self.assertIs(result["completion"]["explicit_finish"], False)
                m._failure_graph(result["failure"])
                if mode != "preyield":
                    self.assertIs(result["process_exit"]["cleanup_confirmed"], False)
        _, _, output = self.worker_cut("tick1", reporting_failure=True)
        self.assertFalse((output / "result.json").exists())
        self.assertEqual(m.read(output / "steps.jsonl").count(b"\n"), 1)

    def test_exact_artifact_roster_rejects_omission_and_extra(self):
        output, result = self.run_case("body")
        for name in (
            "binding.json",
            "steps.jsonl",
            "diagnostics.bin",
            "wire.bin",
            "payload-00000.bin",
        ):
            changed = copy.deepcopy(result)
            del changed["artifacts"][name]
            (output.path / "result.json").write_bytes(m.canonical(changed))
            with (
                self.subTest(missing=name),
                self.assertRaisesRegex(m.CampaignError, "artifact_roster"),
            ):
                m.verify_arm(
                    self.freeze,
                    self.freeze_digest,
                    "body",
                    output.path,
                    require_owner=False,
                )
        extra = output.write("extra.bin", b"extra")
        result["artifacts"]["extra.bin"] = extra
        (output.path / "result.json").write_bytes(m.canonical(result))
        with self.assertRaisesRegex(m.CampaignError, "artifact_roster"):
            m.verify_arm(
                self.freeze,
                self.freeze_digest,
                "body",
                output.path,
                require_owner=False,
            )

    def test_fault_prefix_rejects_foreign_exports_and_false_success(self):
        freeze, case_id, output = self.worker_cut("renderer")
        result = m.verify_fault_case(
            freeze, self.freeze_digest, case_id, output, require_owner=False
        )
        self.assertIs(result["healthy"], False)
        self.assertIs(result["application_cleanup_confirmed"], False)
        self.assertIs(result["partial_capture_is_authenticated"], False)
        original = m.read(output / "steps.jsonl")
        row = m.parse(original)
        payload = output / row["readings"][0]["payload"]["path"]
        before = m.read(payload)
        payload.write_bytes(before[:-1] + bytes([before[-1] ^ 1]))
        changed = m.file_identity(payload)
        row["readings"][0]["payload"] = {**changed, "path": payload.name}
        (output / "steps.jsonl").write_bytes(m.canonical(row) + b"\n")
        fake_output = SimpleNamespace(path=output)
        self.rehash(fake_output, payload.name)
        self.rehash(fake_output, "steps.jsonl")
        with self.assertRaises(ValueError):
            m.verify_fault_case(
                freeze, self.freeze_digest, case_id, output, require_owner=False
            )
        payload.write_bytes(before)
        (output / "steps.jsonl").write_bytes(original)
        self.rehash(fake_output, payload.name)
        self.rehash(fake_output, "steps.jsonl")
        m.verify_fault_case(
            freeze, self.freeze_digest, case_id, output, require_owner=False
        )
        with (output / "run.jsonl").open("ab") as stream:
            stream.write(
                m.canonical({"type": "run_ended", "status": "succeeded"}) + b"\n"
            )
        self.rehash(fake_output, "run.jsonl")
        with self.assertRaisesRegex(m.CampaignError, "false_terminal_claim"):
            m.verify_fault_case(
                freeze, self.freeze_digest, case_id, output, require_owner=False
            )

    def test_native_readback_provenance_controls_reuse_original_public_frames(self):
        selection = SelectionControls()
        selection.setUp()
        self.addCleanup(selection.doCleanups)
        self.freeze = selection.freeze
        self.freeze["output_root"] = str(self.root)
        self.freeze["runtime"]["source_identity"] = "a" * 64
        self.freeze_digest = selection.write_freeze(self.freeze)
        for arm in ("body", "canonical"):
            output, result = self.run_case(arm)
            observation = output.write("observer.json", b"synthetic boundary fixture\n")
            output.json(
                "owner.json",
                {
                    "schema": "prisoma.m1-owner.v1",
                    "freeze_sha256": self.freeze_digest,
                    "case_id": arm,
                    "binding_sha256": result["binding_sha256"],
                    "result_sha256": m.digest(m.read(output.path / "result.json")),
                    "worker_returncode": 0,
                    "observed_births": [[100, 1, 0]],
                    "retired_births": [[100, 1, 0]],
                    "emergency_cleanup": False,
                    "observer_events": observation,
                    "observer_sha256": self.freeze["tools"]["observer_binary"][
                        "sha256"
                    ],
                },
            )
            (self.root / (arm + ".command.json")).write_bytes(b"{}\n")
        actual_verify = m.verify_arm

        def selected_readback(*args, **kwargs):
            # Synthetic protocol peers have no native ownership claim. Process
            # and command acceptance has its own actual owned-child controls.
            return actual_verify(*args, **kwargs, require_owner=False)

        with (
            patch.object(m, "verify_arm", side_effect=selected_readback),
            patch.object(
                m, "verify_command", return_value={"stdout": {}, "stderr": {}}
            ),
            patch.object(m, "_git_state"),
            patch.object(m, "environment_selection"),
            patch.object(m, "runtime_selection", return_value=None),
        ):
            report = m.provenance_negatives(
                selection.path, "body", "canonical", self.root / "controls"
            )
        self.assertEqual(len(report["controls"]), 13)
        self.assertEqual(report["controls"]["mixed-wheel-freeze"], "wheel_distribution")
        self.assertEqual(
            report["controls"]["matching-altered-workload-body"], "original_request"
        )
        self.assertEqual(
            report["controls"]["matching-altered-workload-canonical"],
            "original_request",
        )
        self.assertTrue(report["original_bytes_unchanged"])


class DiagnosticAndCheckpointTests(unittest.TestCase):
    def test_hostile_diagnostics_shared_edges_and_cycles_are_bounded(self):
        class HostileError(RuntimeError):
            @property
            def __class__(self):
                raise AssertionError("class callback")

            def __str__(self):
                raise AssertionError("format callback")

        first = HostileError("a" * 10000)
        first.__cause__ = first
        root = BaseExceptionGroup("shared", [first] * 20000)
        value = m.diagnostic(root, node_limit=4, edge_limit=8)
        self.assertTrue(value["truncated"])
        self.assertEqual(len(value["nodes"]), 2)
        self.assertTrue(value["nodes"][1]["fields_truncated"])
        self.assertLessEqual(len(value["edges"]), 8)
        self.assertTrue(any(row["repeated"] for row in value["edges"]))
        healthy = m.diagnostic(RuntimeError("ordinary"))
        self.assertFalse(healthy["truncated"])
        self.assertEqual(healthy["nodes"][0]["messages"], ["ordinary"])

    def test_deep_nested_cancellation_hostile_group_and_two_node_cycle(self):
        class HostileGroup(BaseExceptionGroup):
            @property
            def exceptions(self):
                raise AssertionError("group callback")

        first, second = KeyboardInterrupt("cancelled"), RuntimeError("cleanup")
        first.__cause__, second.__context__ = second, first
        root = HostileGroup("nested", [first, second])
        for _ in range(5000):
            root = BaseExceptionGroup("deep", [root])
        result = m.diagnostic(root)
        self.assertTrue(result["truncated"])
        self.assertEqual(len(result["nodes"]), 128)
        m._failure_graph(result)
        result = m.diagnostic(HostileGroup("cycle", [first, second]))
        self.assertFalse(result["truncated"])
        self.assertEqual(len(result["nodes"]), 3)
        self.assertTrue(any(edge["repeated"] for edge in result["edges"]))
        m._failure_graph(result)

    def test_checkpoint_accepts_only_exact_bounded_continue(self):
        for mode in (
            "valid",
            "generation",
            "generation-float",
            "generation-bool",
            "eof",
            "oversized",
            "duplicate",
            "stage",
            "multiline",
        ):
            with self.subTest(mode=mode):
                progress_read, progress_write = os.pipe()
                control_read, control_write = os.pipe()
                binding = {
                    "freeze_sha256": "a" * 64,
                    "case": {"case_id": "test"},
                    "binding": {**c.raw(new_binding()), "generation": 1},
                }

                def controller():
                    with os.fdopen(progress_read, "rb") as stream:
                        row = m.parse(stream.readline(m.MAX_CHECKPOINT + 1))
                    row["schema"] = "prisoma.m1-continue.v1"
                    if mode == "generation":
                        row["generation"] = 2
                    elif mode == "generation-float":
                        row["generation"] = 1.0
                    elif mode == "generation-bool":
                        row["generation"] = True
                    elif mode == "stage":
                        row["stage"] = "tick1"
                    raw = (
                        b"x" * (m.MAX_CHECKPOINT + 1)
                        if mode == "oversized"
                        else m.canonical(row) + b"\n"
                    )
                    if mode == "duplicate":
                        raw = raw.replace(
                            b'"generation":1', b'"generation":1,"generation":1'
                        )
                    elif mode == "multiline":
                        raw += b"{}\n"
                    try:
                        if mode != "eof":
                            os.write(control_write, raw)
                    finally:
                        os.close(control_write)

                thread = threading.Thread(target=controller)
                thread.start()
                try:
                    if mode == "valid":
                        m.checkpoint(
                            binding,
                            "prepared",
                            progress_write,
                            control_read,
                            time.monotonic() + 5,
                            2,
                        )
                    else:
                        with self.assertRaises(m.CampaignError):
                            m.checkpoint(
                                binding,
                                "prepared",
                                progress_write,
                                control_read,
                                time.monotonic() + 5,
                                2,
                            )
                finally:
                    os.close(progress_write)
                    os.close(control_read)
                    thread.join(3)
                    self.assertFalse(thread.is_alive())


class SelectionControls(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name).resolve()
        script = m.file_identity(SCRIPT)
        self.freeze = {
            "schema": "prisoma.m1-freeze.v1",
            "campaign_id": "schema-control",
            "workload": m.file_identity(WORKLOAD),
            "source": {
                "root": str(SCRIPT.parents[3]),
                "commit": "1" * 40,
                "tree": "2" * 40,
            },
            "tools": {
                role: script
                for role in (
                    "worker",
                    "runner",
                    "supervisor",
                    "observer_source",
                    "observer_python",
                    "observer_binary",
                    "fault_source",
                    "fault_binary",
                )
            },
            "renderer": script,
            "renderer_parent": script,
            "runtime": {
                "prefix": str(self.root),
                "manifest_sha256": "a" * 64,
                "source_identity": "b" * 64,
            },
            "limits": {"session_seconds": 180, "checkpoint_seconds": 30},
            "output_root": str(self.root),
            "environments": {},
            "cases": [
                {"case_id": arm, "arm": arm, "fault": "none"}
                for arm in ("body", "canonical")
            ],
        }
        wheels, distributions = {}, {}
        for package in sorted(m.CANONICAL_DISTRIBUTIONS):
            wheel = self.root / (package + ".whl")
            name = package.replace("-", "_") + "-0.dist-info/METADATA"
            raw = f"Name: {package}\nVersion: 0\n".encode()
            payload_name = package.replace("-", "_") + "/__init__.py"
            with zipfile.ZipFile(wheel, "w") as archive:
                archive.writestr(name, raw)
                archive.writestr(payload_name, b"# schema-only fixture\n")
            wheels[package] = m.file_identity(wheel)
            distributions[package] = {
                "version": "0",
                "files": {
                    "lib/python3.11/site-packages/" + name: {
                        "bytes": len(raw),
                        "sha256": m.digest(raw),
                    },
                    "lib/python3.11/site-packages/" + payload_name: {
                        "bytes": 22,
                        "sha256": m.digest(b"# schema-only fixture\n"),
                    },
                },
            }
        for arm, packages in (
            ("body", m.SHARED_DISTRIBUTIONS),
            ("canonical", m.CANONICAL_DISTRIBUTIONS),
        ):
            prefix = str(self.root / arm)
            path = self.root / (arm + "-inventory.json")
            path.write_bytes(
                m.canonical(
                    {
                        "schema": "prisoma.m1-installed-inventory.v1",
                        "prefix": prefix,
                        "distributions": {
                            package: distributions[package] for package in packages
                        },
                    }
                )
            )
            self.freeze["environments"][arm] = {
                "prefix": prefix,
                "inventory": m.file_identity(path),
                "wheels": {package: wheels[package] for package in packages},
            }
        self.path = self.root / "freeze.json"

    def write_freeze(self, value=None):
        self.path.write_bytes(m.canonical(self.freeze if value is None else value))
        return m.digest(m.read(self.path))

    def test_closed_freeze_matches_shared_artifacts_and_rejects_drift(self):
        self.write_freeze()
        self.assertEqual(m.load_freeze(self.path, verify_source=False)[0], self.freeze)
        other = self.root / "other.whl"
        other.write_bytes(b"different schema-only wheel fixture")
        changes = [
            lambda row: row["tools"].pop("runner"),
            lambda row: row["limits"].update(session_seconds=True),
            lambda row: row["cases"].append(row["cases"][0]),
            lambda row: row["environments"]["canonical"]["wheels"].update(
                {"ncp-local": m.file_identity(other)}
            ),
            lambda row: row["tools"].update(observer_source=m.file_identity(other)),
            lambda row: row["workload"].update(sha256="0" * 64),
        ]
        for index, change in enumerate(changes):
            with self.subTest(change=index):
                selected = copy.deepcopy(self.freeze)
                change(selected)
                self.write_freeze(selected)
                with self.assertRaises(m.CampaignError):
                    m.load_freeze(self.path, verify_source=False)
        self.write_freeze()
        m.load_freeze(self.path, verify_source=False)

    def test_source_observation_ignores_ambient_git_redirection(self):
        repository = self.root / "source"
        repository.mkdir()
        environment = {
            key: value
            for key, value in os.environ.items()
            if not key.startswith("GIT_")
        }

        def git(*args):
            return (
                subprocess.check_output(
                    ["git", "-C", str(repository), *args], env=environment
                )
                .decode()
                .strip()
            )

        git("init", "-q")
        (repository / "file").write_text("selected source\n")
        git("add", "file")
        git(
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "source fixture",
        )
        source = {
            "root": str(repository),
            "commit": git("rev-parse", "HEAD"),
            "tree": git("rev-parse", "HEAD^{tree}"),
        }
        with patch.dict(
            os.environ,
            {"GIT_DIR": str(self.root / "missing"), "GIT_WORK_TREE": str(self.root)},
        ):
            m._git_state(source)
        (repository / "untracked").write_bytes(b"drift")
        with self.assertRaisesRegex(m.CampaignError, "source_changed"):
            m._git_state(source)
        (repository / "untracked").unlink()
        m._git_state(source)

    def test_selected_wheel_cannot_omit_or_change_installed_payload(self):
        environment = self.freeze["environments"]["body"]
        inventory = m.parse(m.selected_file(environment["inventory"]))
        wheel = environment["wheels"]["ncp-local"]
        m.wheel_installation("ncp-local", wheel, inventory)
        with zipfile.ZipFile(wheel["path"]) as archive:
            entries = {row.filename: archive.read(row) for row in archive.infolist()}
        payload = next(name for name in entries if name.endswith("/__init__.py"))
        for mode in ("omitted", "changed"):
            path = self.root / (mode + ".whl")
            with zipfile.ZipFile(path, "w") as archive:
                for name, raw in entries.items():
                    if name == payload and mode == "omitted":
                        continue
                    archive.writestr(
                        name, raw + b"# drift\n" if name == payload else raw
                    )
            with self.assertRaisesRegex(m.CampaignError, "wheel_installation"):
                m.wheel_installation("ncp-local", m.file_identity(path), inventory)
        m.wheel_installation("ncp-local", wheel, inventory)

    def test_external_command_receipt_requires_exact_success_and_owner(self):
        freeze_digest = self.write_freeze()
        case = self.freeze["cases"][0]
        output = m.Output(self.root / "body")
        output.json("owner.json", {"fixture": "schema-only; no process claim"})
        logs = m.Output(self.root / "body.command")
        logs.write("stdout.bin", b"")
        logs.write("stderr.bin", b"")
        value = {
            "schema": "prisoma.m1-command.v1",
            "freeze": m.file_identity(self.path),
            "case": case,
            "runner": self.freeze["tools"]["runner"],
            "supervisor": self.freeze["tools"]["supervisor"],
            "argv": [
                str(self.root / "body/bin/python"),
                "-I",
                "-B",
                str(SCRIPT),
                "--freeze",
                str(self.path),
                "--case",
                "body",
            ],
            "limits": {
                "wall_seconds": 270,
                "grace_seconds": 30,
                "stream_bytes": 925696,
            },
            "returncode": 0,
            "timed_out": False,
            "output_overflow": False,
            "interrupted": False,
            "forced_kill": False,
            "failure": None,
            "stdout": m.file_identity(logs.path / "stdout.bin"),
            "stderr": m.file_identity(logs.path / "stderr.bin"),
            "owner": m.file_identity(output.path / "owner.json"),
        }
        path = self.root / "body.command.json"
        path.write_bytes(m.canonical(value))
        m.verify_command(self.path, self.freeze, freeze_digest, "body")
        for key, replacement in (
            ("returncode", False),
            ("returncode", 1),
            ("timed_out", True),
            ("output_overflow", True),
            ("interrupted", True),
            ("forced_kill", True),
            ("failure", "lost"),
            ("owner", None),
            ("argv", value["argv"] + ["extra"]),
            ("case", self.freeze["cases"][1]),
        ):
            path.write_bytes(m.canonical({**value, key: replacement}))
            with (
                self.subTest(key=key, replacement=replacement),
                self.assertRaises(m.CampaignError),
            ):
                m.verify_command(self.path, self.freeze, freeze_digest, "body")
        path.write_bytes(m.canonical(value))
        m.verify_command(self.path, self.freeze, freeze_digest, "body")

    def test_observation_accepts_actual_tracker_pid_reuse_and_status_changes(self):
        spec = importlib.util.spec_from_file_location(
            "m1_observer_control", SCRIPT.with_name("owned_observer.py")
        )
        observer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(observer)

        def row(pid, ppid, seconds):
            return {
                "pid": pid,
                "ppid": ppid,
                "uid": os.getuid(),
                "status": 2,
                "start_seconds": seconds,
                "start_microseconds": 0,
            }

        root, old, replacement, child = (
            row(100, os.getpid(), 10),
            row(101, 100, 11),
            row(101, 100, 12),
            row(102, 101, 13),
        )
        tree = observer.OwnedTree(
            None,
            SimpleNamespace(pid=100, returncode=None),
            ({100: root, 101: old}, set()),
        )
        tree.update(({100: root, 101: replacement, 102: child}, set()))
        observation = tree.receipt(({}, set()))
        value = {
            "schema": "prisoma.m1-owner-observation.v1",
            "freeze_sha256": "a" * 64,
            "case_id": "body",
            "observation": observation,
            "supervisor_events": [
                {"kind": "checkpoint", "stage": stage, "binding_sha256": "b" * 64}
                for stage in ("prepared", "tick1")
            ],
        }
        path = self.root / "observer.json"
        path.write_bytes(m.canonical(value))
        selected = m.file_identity(path)
        owner = {
            "observed_births": [list(identity) for identity in tree.known],
            "retired_births": [list(identity) for identity in tree.absent],
            "observer_events": {**selected, "path": path.name},
        }
        m._observation(
            self.freeze, "a" * 64, self.freeze["cases"][0], "b" * 64, owner, self.root
        )
        case = {"case_id": "renderer", "arm": "canonical", "fault": "renderer_loss"}
        current = {**child, "status": 3}
        value["case_id"] = case["case_id"]
        value["supervisor_events"] += [
            {
                "kind": "renderer_fault_intent",
                "identity": current,
                "executable": self.freeze["renderer"],
                "parent_identity": replacement,
                "parent_executable": self.freeze["renderer_parent"],
                "signal": 15,
            },
            {"kind": "renderer_fault_return", "returncode": 0},
            {
                "kind": "renderer_fault_observed",
                "receipt": {
                    "schema": "local.darwin-version-bound-fault.v1",
                    "pid": 102,
                    "start_seconds": 13,
                    "start_microseconds": 0,
                    "ppid": 101,
                    "pid_version": 123,
                    "signal": 15,
                    "signal_result": 0,
                },
            },
        ]
        for invalid in (False, True):
            if invalid:
                value["supervisor_events"][2]["parent_identity"] = {
                    **replacement,
                    "status": 5,
                }
            raw = m.canonical(value)
            path.write_bytes(raw)
            owner["observer_events"] = {
                "path": path.name,
                "bytes": len(raw),
                "sha256": m.digest(raw),
            }
            if invalid:
                with self.assertRaises(m.CampaignError):
                    m._observation(
                        self.freeze, "a" * 64, case, "b" * 64, owner, self.root
                    )
            else:
                m._observation(self.freeze, "a" * 64, case, "b" * 64, owner, self.root)


if __name__ == "__main__":
    unittest.main()
