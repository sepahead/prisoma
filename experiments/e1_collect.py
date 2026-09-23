"""Collect the fixed E1 study through public canonical checkpoint-family operations.

A caller must separately freeze and qualify the installed runtime and full plans.
Only complete original-capture readback can produce a derived episode. This module
changes no numerical threshold and grants no loaded-byte or release attestation.
"""

from __future__ import annotations

import base64
from dataclasses import asdict, dataclass
import hashlib
import os
from pathlib import Path
import re
import struct

from crebain_ncp_sensors import codec as c, family_contract as fc, family_types as f
from crebain_ncp_sensors import types as t
from prisoma_agent_bridge import crebain_family as family

from experiments import e1_reference as ref


class CollectionError(ValueError):
    """A failed study or canonical-to-numerical join."""


def _require(condition, code):
    if not condition:
        raise CollectionError(code)


def _same(left, right):
    return ref.canonical(c.raw(left)) == ref.canonical(c.raw(right))


def _sha(payload):
    return hashlib.sha256(payload).hexdigest()


@dataclass(frozen=True, slots=True)
class StudyCase:
    episode_id: str
    plan: f.FamilyPlan


@dataclass(frozen=True, slots=True)
class CollectedEpisode:
    evaluation: ref.EpisodeEvaluation
    receipt: dict


@dataclass(frozen=True, slots=True)
class CollectedQualification:
    """A qualification receipt, excluded from Reference.Episode and model scores."""

    receipt: dict


@dataclass(frozen=True, slots=True)
class _QualificationCommitment:
    episode_id: str
    input_sha256: str
    persistence_pa: float
    selected_action: str = "neutral"

    def record(self):
        return {
            "schema": "prisoma.e1-qualification-commitment.v1",
            "contract_sha256": ref.content_digest(ref.study_contract()),
            "episode_id": self.episode_id,
            "stage": "qualification",
            "input_sha256": self.input_sha256,
            "action_order": ref.ACTIONS,
            "forecasts": [
                {"predictor_id": "persistence", "values_pa": (self.persistence_pa,) * 5}
            ],
            "collection_policy": "neutral",
            "selected_action": self.selected_action,
            "model_selection_sha256": None,
            "scope": "qualification_excluded_from_training_and_model_score",
        }

    @property
    def sha256(self):
        return ref.content_digest(self.record())


def _row(episode_id, *, qualification=False):
    rows = [
        row for row in ref.seed_roster()["episodes"] if row["episode_id"] == episode_id
    ]
    _require(
        len(rows) == 1 and (rows[0]["split"] == "qualification") is qualification,
        "qualification_split" if qualification else "study_split",
    )
    return rows[0]


def _targets():
    return tuple(
        t.SetTarget("set_target", True, row["roll_rad"], row["pitch_rad"], 0.0, 8.0)
        for row in ref.study_contract()["actions"]
    )


def validate_case(case, *, qualification=False):
    """Admit study-owned fields; the external freeze owns the remaining physics."""
    _require(type(case) is StudyCase, "typed_study_case")
    row = _row(case.episode_id, qualification=qualification)
    plan = fc.freeze_plan(case.plan)
    spec, study = plan.body.specification, ref.study_contract()
    scene = spec.scene
    _require(plan.body.planned_ticks == 36 and plan.landmark_tick == 12, "study_ticks")
    _require(
        spec.seed == row["seeds"]["runtime"]
        and spec.acoustic.seed == row["seeds"]["acoustic"]
        and len(spec.drones) == 1
        and spec.drones[0].position == tuple(row["explicit_position_m"]),
        "study_seeds_position",
    )
    _require(
        scene.frame == "three-y-up-z-forward-m"
        and scene.solids == ()
        and len(scene.rgbCameras) == 2
        and scene.thermalCameras == ()
        and len(scene.microphones) == 1,
        "study_scene_roster",
    )
    for camera, position in zip(
        scene.rgbCameras, study["scene"]["camera_positions_m"], strict=True
    ):
        _require(
            camera.position == tuple(position)
            and camera.target == tuple(study["scene"]["camera_target_m"])
            and (camera.width, camera.height, camera.periodTicks, camera.fovDegrees)
            == (160, 120, 3, 60),
            "study_camera",
        )
    _require(
        scene.microphones[0].position == (0, 2, 10)
        and spec.acoustic.sampleRateHz == 16000
        and spec.controller.referenceHeadingRad == 0
        and spec.controller.referenceAltitudeM == 8,
        "study_pressure_controller",
    )
    _require(
        len(plan.branches) == 5
        and all(branch.purpose == "label" for branch in plan.branches)
        and all(
            _same(branch.target, target)
            for branch, target in zip(plan.branches, _targets(), strict=True)
        ),
        "study_action_roster",
    )
    catalog = c.expected_catalog(spec)
    cameras = tuple(entry.sensor_id for entry in catalog if entry.kind == "rgba8")
    pressure = tuple(entry.sensor_id for entry in catalog if entry.kind == "pressure")
    _require(len(cameras) == 2 and len(pressure) == 1, "study_catalog")
    _require(
        _same(
            plan.evaluation,
            f.PressureWindow(
                "scaled_compensated_pressure_rms400_v1",
                pressure[0],
                34,
                36,
                400,
                "pascal",
                ref.TARGET_FUNCTION_SHA256,
            ),
        ),
        "study_target",
    )
    family.capture_budget(plan)  # Public admission precedes output or runtime effects.
    return StudyCase(case.episode_id, plan), row, cameras, pressure[0]


def _stage_inputs(row, references, selection):
    _require(type(references) is tuple, "stage_artifacts")
    stage = row["split"]
    if stage in ("train", "qualification"):
        _require(
            not references and selection is None,
            "qualification_order" if stage == "qualification" else "training_order",
        )
    elif stage == "development":
        _require(selection is None and len(references) == 4, "development_order")
        _require(
            all(type(item) is ref.Reference for item in references), "development_order"
        )
        _require(
            tuple(item.penalty for item in references) == ref.LAMBDAS,
            "development_order",
        )
    else:
        _require(not references and type(selection) is ref.Selection, "holdout_lock")


def _commit(row, landmark, references, selection):
    if row["split"] == "qualification":
        return _QualificationCommitment(
            row["episode_id"], landmark.input_sha256, landmark.values[7]
        )
    if row["split"] == "heldout":
        return ref.commit_selected(selection, row["episode_id"], landmark)
    return ref.commit_collection(row["split"], row["episode_id"], landmark, references)


def _pressure(reading):
    manifest, tensor = reading.manifest, reading.manifest.tensor
    _require(type(tensor) is t.PressureTensor, "pressure_tensor")
    return ref.PressureBlock(
        manifest.sensor_id,
        manifest.source_body_tick,
        manifest.available_after_body_tick,
        tensor.sample_start,
        tensor.sample_end,
        reading.payload,
        tensor.shape,
        tensor.kind,
        tensor.dtype,
        tensor.layout,
        tensor.sample_rate_hz,
        tensor.unit,
    )


def _rgb(reading):
    manifest, tensor = reading.manifest, reading.manifest.tensor
    _require(type(tensor) is t.RgbaTensor, "rgb_tensor")
    return ref.RGBFrame(
        manifest.sensor_id,
        manifest.source_body_tick,
        manifest.available_after_body_tick,
        reading.payload,
        tensor.kind,
        tensor.dtype,
        tensor.layout,
        tensor.encoding,
        tensor.row_origin,
        tensor.shape,
    )


class _Windows:
    """Retain only the admitted landmark or final window, never whole trajectories."""

    def __init__(self, cameras, microphone, *, landmark=False):
        self.cameras, self.microphone, self.landmark = cameras, microphone, landmark
        self.frames, self.readings = {}, []
        self.final = None

    def observe(self, step):
        _require(type(step) is family.FamilyObservation, "typed_observation")
        tick = step.response.body.tick
        _require(
            step.observation.batch == step.response.body.batch, "observation_batch"
        )
        first = 10 if self.landmark else 34
        if not first <= tick <= first + 2:
            return
        for reading in step.observation.readings:
            manifest = reading.manifest
            c.validate_payload(manifest, reading.byte_manifest, reading.payload)
            _require(
                manifest.source_body_tick == manifest.available_after_body_tick == tick,
                "observation_tick",
            )
            if manifest.sensor_id == self.microphone:
                _require(len(self.readings) == tick - first, "pressure_window_order")
                self.readings.append(reading)
            elif self.landmark and tick == 12 and manifest.sensor_id in self.cameras:
                _require(manifest.sensor_id not in self.frames, "duplicate_frame")
                self.frames[manifest.sensor_id] = reading
        self.final = step.response

    def blocks(self):
        return tuple(_pressure(reading) for reading in self.readings)

    def features(self):
        _require(set(self.frames) == set(self.cameras), "landmark_frames")
        return ref.landmark_features(
            tuple(_rgb(self.frames[name]) for name in self.cameras),
            self.blocks(),
            camera_ids=self.cameras,
            microphone_id=self.microphone,
        )

    def target(self):
        values, digest = ref.pressure_window(
            self.blocks(), first_tick=34, source_id=self.microphone
        )
        return (
            ref.pressure_rms(values),
            digest,
            b"".join(item.payload for item in self.readings),
        )


class _Readback:
    """Provisional visitor; only inspect_family_run can authorize its final use."""

    def __init__(self, case, row, cameras, microphone, references, selection):
        self.case, self.row = case, row
        self.references, self.selection = references, selection
        self.landmark = _Windows(cameras, microphone, landmark=True)
        self.windows = tuple(_Windows(cameras, microphone) for _ in range(6))
        self.prepared = self.checkpoint = self.commitment = self.decision = (
            self.finished
        ) = None
        self.labels = []
        self.original_forecast = None

    def visit(self, event):
        plan = self.case.plan
        method, result, slot = event.method.rsplit(".", 1)[-1], event.result, event.slot
        if method == "prepare":
            _require(
                ref.canonical(event.payload) == ref.canonical(c.raw(plan)),
                "replayed_plan",
            )
            _require(
                type(result) is f.FamilyPrepared
                and result.family_plan_digest
                == fc.plan_digest(plan, result.body.source_identity),
                "prepared_plan",
            )
            self.prepared = result
        elif method == "advance":
            _require(
                type(result) is family.FamilyObservation and result.slot == slot,
                "observation_slot",
            )
            tick = result.response.body.tick
            expected = None
            if slot == 0 and tick == 1:
                expected = _targets()[0]
            elif tick == 13:
                _require(self.commitment is not None, "forecast_before_execution")
                index = (
                    ref.ACTIONS.index(self.commitment.selected_action)
                    if slot == 0
                    else slot - 1
                )
                expected = plan.branches[index].target
            _require(_same(result.requested_target, expected), "executed_target")
            if slot == 0 and tick <= 12:
                self.landmark.observe(result)
            self.windows[slot].observe(result)
        elif method == "checkpoint":
            _require(type(result) is f.Checkpointed, "typed_checkpoint")
            self.landmark.features()
            self.checkpoint = result
        elif method == "commit_decision":
            _require(
                self.checkpoint is not None and self.commitment is None,
                "commitment_order",
            )
            self.commitment = _commit(
                self.row, self.landmark.features(), self.references, self.selection
            )
            original = base64.b64decode(event.payload["forecast_base64"], validate=True)
            expected = ref.canonical(self.commitment.record())
            branch = plan.branches[ref.ACTIONS.index(self.commitment.selected_action)]
            _require(
                original == expected and _sha(original) == self.commitment.sha256,
                "original_forecast",
            )
            _require(
                type(result) is f.DecisionCommitted
                and _same(result.checkpoint, self.checkpoint.reference)
                and result.forecast_commitment_digest == self.commitment.sha256
                and result.selected_case_id
                == event.payload["selected_case_id"]
                == branch.case_id
                and _same(result.selected_target, branch.target),
                "decision_join",
            )
            self.original_forecast, self.decision = original, result
        elif method == "evaluate":
            _require(
                self.commitment is not None and slot == len(self.labels) + 1,
                "label_order",
            )
            self.labels.append(self._label(slot, result))
        elif method == "finish":
            self.finished = result

    def _label(self, slot, result):
        plan, window = self.case.plan, self.windows[slot]
        _require(
            type(result) is f.EvaluationResult and window.final is not None,
            "typed_label",
        )
        value, digest, _ = window.target()
        segments = tuple(
            f.PressureSegment(
                item.manifest.source_body_tick,
                item.manifest.available_after_body_tick,
                item.manifest.tensor.sample_start,
                item.manifest.tensor.sample_end,
                item.manifest.manifest_digest,
                item.manifest.byte_manifest_digest,
                _sha(item.payload),
                len(item.payload),
            )
            for item in window.readings
        )
        branch = plan.branches[slot - 1]
        _require(
            _same(result.target, plan.evaluation)
            and _same(result.segments, segments)
            and result.window_payload_sha256 == digest
            and struct.pack("<d", result.value_pa) == struct.pack("<d", value)
            and result.scientific_validation is False,
            "original_label_window",
        )
        ancestry, final = result.ancestry, window.final.body
        _require(
            ancestry.family_id == plan.family_id
            and ancestry.case_id == branch.case_id
            and _same(ancestry.origin, self.checkpoint.reference)
            and (
                not self.labels
                or ancestry.origin_plan_digest
                == self.labels[0].ancestry.origin_plan_digest
            )
            and _same(ancestry.execution_binding, branch.binding)
            and _same(ancestry, window.final.ancestry)
            and result.final_native_batch_sha256 == final.batch.engine_batch_sha256
            and result.final_sensor_batch_digest == final.batch.batch_digest
            and result.accepted_action_request_digest
            == final.accepted_action_request_digest,
            "label_ancestry",
        )
        return result

    def accepted(self, verification):
        _require(
            verification["family_replayed"] is True
            and verification["family_id"] == self.case.plan.family_id
            and len(self.labels) == 5
            and self.finished is not None,
            "complete_family_readback",
        )
        selected = ref.ACTIONS.index(self.commitment.selected_action)
        canonical = self.windows[0]
        selected_window = self.windows[selected + 1]
        actual, actual_digest, actual_bytes = canonical.target()
        restored, restored_digest, restored_bytes = selected_window.target()
        _require(
            actual_bytes == restored_bytes
            and actual_digest == restored_digest
            and actual == restored,
            "selected_original_bytes",
        )
        final = canonical.final.canonical_final_state
        _require(
            type(final) is f.CanonicalFinalState
            and final.cpu_state_sha256 == self.labels[selected].final_cpu_state_sha256
            and self.finished["terminal"]["canonical_final_state"] == c.raw(final),
            "selected_final_state",
        )
        labels = c.raw(tuple(self.labels))
        receipt = {
            "schema": "prisoma.e1-collected-episode.v1",
            "episode_id": self.case.episode_id,
            "split": self.row["split"],
            "seed_row": self.row,
            "plan_content_sha256": ref.content_digest(c.raw(self.case.plan)),
            "family_plan_digest": self.prepared.family_plan_digest,
            "original_forecast_base64": base64.b64encode(self.original_forecast).decode(
                "ascii"
            ),
            "forecast_sha256": self.commitment.sha256,
            "decision": c.raw(self.decision),
            "checkpoint": c.raw(self.checkpoint),
            "labels": labels,
            "finish": self.finished,
            "verification": verification,
            "policy_comparison": {
                "selected_action": self.commitment.selected_action,
                "canonical_selected_rms_pa": actual,
                "canonical_selected_payload_sha256": actual_digest,
                "matched_restored_neutral_rms_pa": self.labels[0].value_pa,
                "matched_restored_neutral_payload_sha256": self.labels[
                    0
                ].window_payload_sha256,
                "neutral_minus_selected_pa": self.labels[0].value_pa - actual,
                "selected_restored_original_bytes_equal": True,
                "scope": "paired_simulator_execution_not_forecast_accuracy",
            },
            "authority": {
                "canonical_original_capture_replayed": True,
                "native_qualification_verified_here": False,
                "loaded_bytes_attested": False,
                "scientific_validation": False,
                "release_qualified": False,
            },
        }
        if self.row["split"] == "qualification":
            return CollectedQualification(receipt)
        episode = ref.Episode(
            self.case.episode_id,
            self.landmark.features(),
            tuple(item.value_pa for item in self.labels),
            ref.content_digest(labels),
        )
        return CollectedEpisode(ref.evaluate_episode(self.commitment, episode), receipt)


def _readback_episode(
    case, log_path, *, references=(), selection=None, qualification=False
):
    """Derive only after public canonical and all journal terminals succeed."""
    case, row, cameras, microphone = validate_case(case, qualification=qualification)
    _stage_inputs(row, references, selection)
    visitor = _Readback(case, row, cameras, microphone, references, selection)
    verification = family.inspect_family_run(log_path, visitor.visit)
    return visitor.accepted(verification)


def _publish(path, value):
    payload = ref.canonical(value)
    _require(len(payload) <= 1_048_576, "derived_artifact_bound")
    stream, errors = None, []
    try:
        stream = path.open("xb")
        stream.write(payload)
        stream.flush()
        os.fsync(stream.fileno())
    except BaseException as error:
        errors.append(error)
    finally:
        if stream is not None:
            try:
                stream.close()
            except BaseException as error:
                errors.append(error)
    if len(errors) == 1:
        raise errors[0]
    if errors:
        raise BaseExceptionGroup("derived artifact and cleanup failed", errors)
    _require(path.read_bytes() == payload, "derived_artifact_readback")
    return {"name": path.name, "bytes": len(payload), "sha256": _sha(payload)}


def _collect_episode(
    runtime, case, output, *, references=(), selection=None, qualification=False
):
    """Execute once; incomplete originals remain on failure, without replacement."""
    case, row, cameras, microphone = validate_case(case, qualification=qualification)
    _stage_inputs(row, references, selection)
    output = Path(output)
    output.mkdir(mode=0o700)
    _publish(
        output / "case.json",
        {"episode_id": case.episode_id, "seed_row": row, "plan": c.raw(case.plan)},
    )
    landmark = _Windows(cameras, microphone, landmark=True)
    log = output / "run.jsonl"
    with family.owned_family_experiment(runtime, case.plan, log) as experiment:
        for tick in range(1, 13):
            landmark.observe(experiment.advance(_targets()[0] if tick == 1 else None))
        experiment.checkpoint()
        commitment = _commit(row, landmark.features(), references, selection)
        original = ref.canonical(commitment.record())
        branch = case.plan.branches[ref.ACTIONS.index(commitment.selected_action)]
        experiment.commit_decision(original, branch.case_id)
        for tick in range(13, 37):
            experiment.advance(branch.target if tick == 13 else None)
        for branch in case.plan.branches:
            experiment.reserve(branch.case_id)
            experiment.restore(branch.slot)
            for tick in range(13, 37):
                experiment.advance(
                    branch.target if tick == 13 else None, slot=branch.slot
                )
            experiment.evaluate(branch.slot)
            experiment.branch_finish(branch.slot)
        experiment.release_checkpoint()
        experiment.finish()
    result = _readback_episode(
        case,
        log,
        references=references,
        selection=selection,
        qualification=qualification,
    )
    _publish(output / "episode.json", asdict(result))
    return result


def collect_episode(runtime, case, output, *, references=(), selection=None):
    """Execute one admitted training, development, or held-out case exactly once."""
    return _collect_episode(
        runtime, case, output, references=references, selection=selection
    )


def collect_qualification_episode(runtime, case, output):
    """Execute one of eight separate neutral/persistence qualification cases."""
    return _collect_episode(runtime, case, output, qualification=True)


def readback_episode(case, log_path, *, references=(), selection=None):
    """Reconstruct one study episode; qualification inputs remain ineligible."""
    return _readback_episode(case, log_path, references=references, selection=selection)


def readback_qualification_episode(case, log_path):
    """Reconstruct original qualification evidence without fitting or scoring."""
    return _readback_episode(case, log_path, qualification=True)


def collect_study(runtime, cases, output, *, source_sha256, runtime_inventory_sha256):
    """Collect 112 study episodes in order; eight qualification episodes stay external."""
    expected = tuple(
        row for row in ref.seed_roster()["episodes"] if row["split"] != "qualification"
    )
    _require(
        type(cases) is tuple and all(type(case) is StudyCase for case in cases),
        "study_roster",
    )
    _require(
        tuple(case.episode_id for case in cases)
        == tuple(row["episode_id"] for row in expected),
        "study_roster",
    )
    for digest in (source_sha256, runtime_inventory_sha256):
        _require(
            type(digest) is str and re.fullmatch(r"[0-9a-f]{64}", digest) is not None,
            "source_selector",
        )
    admitted = tuple(validate_case(case)[0] for case in cases)
    identities, invariants = [], []
    for case in admitted:
        plan = case.plan
        identities.append(plan.family_id)
        for binding in (
            plan.canonical_binding,
            *(branch.binding for branch in plan.branches),
        ):
            identities.extend((binding.run_id, binding.endpoint_id, binding.generation))
        invariant = c.raw(plan.body.specification)
        del invariant["seed"]
        del invariant["acoustic"]["seed"]
        del invariant["drones"][0]["position"]
        invariants.append(ref.canonical(invariant))
    _require(len(identities) == len(set(identities)), "study_distinct_identities")
    _require(
        all(item == invariants[0] for item in invariants), "study_fixed_environment"
    )
    output = Path(output)
    output.mkdir(mode=0o700)
    _publish(
        output / "study.json",
        {
            "schema": "prisoma.e1-collection-roster.v1",
            "study": ref.study_contract(),
            "seed_roster": ref.seed_roster(),
            "cases": [asdict(case) for case in admitted],
            "source_sha256": source_sha256,
            "runtime_inventory_sha256": runtime_inventory_sha256,
            "qualification_episodes_collected_here": 0,
            "replacement_allowed": False,
        },
    )
    completed, train, development, heldout = [], [], [], []
    grid, selection = (), None
    try:
        for index, case in enumerate(admitted):
            if index == 64:
                grid = ref.fit_reference(
                    tuple(train),
                    source_sha256=source_sha256,
                    runtime_inventory_sha256=runtime_inventory_sha256,
                )
                for number, model in enumerate(grid):
                    _publish(output / f"reference-{number}.json", model.record())
            if index == 80:
                selection = ref.choose_lambda(grid, tuple(development))
                _publish(output / "selection.json", selection.record())
            result = collect_episode(
                runtime,
                case,
                output / case.episode_id,
                references=grid if 64 <= index < 80 else (),
                selection=selection if index >= 80 else None,
            )
            completed.append(case.episode_id)
            if index < 64:
                train.append(result.evaluation.episode)
            elif index < 80:
                development.append(result.evaluation)
            else:
                heldout.append(result.evaluation)
        report = ref.paired_episode_report(selection, tuple(heldout))
        _publish(output / "forecast-report.json", report)
    except BaseException as primary:
        try:
            _publish(
                output / "failure.json",
                {
                    "schema": "prisoma.e1-collection-failure.v1",
                    "completed": completed,
                    "uncompleted": [
                        case.episode_id for case in admitted[len(completed) :]
                    ],
                    "error_type": type(primary).__name__,
                    "replacement_allowed": False,
                },
            )
        except BaseException as secondary:
            raise BaseExceptionGroup(
                "study and failure recording failed", [primary, secondary]
            ) from None
        raise
    return report
