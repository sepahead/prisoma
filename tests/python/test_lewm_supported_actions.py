"""Admission controls plus opt-in actual sklearn arithmetic; no model or simulator.

Run this file with unittest in the qualified LeWM interpreter for the numeric
controls. Ordinary pytest collects the admission controls and skips that class.
"""

import copy
import sys
import unittest
from unittest.mock import patch

import numpy as np

from experiments.lewm import supported_actions as actions
from experiments.lewm.assets import verify_runtime
from experiments.lewm.contracts import StandardizedCandidates


class AdmissionControls(unittest.TestCase):
    def test_row_profile_is_closed_before_copy_or_optional_import(self):
        class Hostile(str):
            def __eq__(self, _other):
                raise AssertionError("Caller equality executed")

        rows = np.zeros((2, 2), np.float32)
        with (
            patch.object(actions, "_snapshot", side_effect=AssertionError("copied")),
            patch.object(actions, "_sklearn", side_effect=AssertionError("imported")),
        ):
            for profile in (None, True, 64 * 1024**2, {}, "64MiB", Hostile("x")):
                with (
                    self.subTest(profile=type(profile)),
                    self.assertRaisesRegex(
                        ValueError, "Unknown action-row admission profile"
                    ),
                ):
                    actions.fit_control_scaler(
                        rows, source_id="profile-control", row_profile=profile
                    )
        for profile in (actions.LEGACY_ROW_PROFILE, actions.COMPLETE_ROW_PROFILE):
            self.assertEqual(
                actions._admit_row_shape((2, 2), 4, profile)["complete_row_bytes"], 16
            )

    def test_complete_byte_budget_checks_dtype_before_copy(self):
        for dtype in (np.float32, np.float64):
            itemsize = np.dtype(dtype).itemsize
            maximum = actions.COMPLETE_ROW_BYTES // (2 * itemsize)
            admitted = actions._admit_row_shape(
                (maximum, 2), itemsize, actions.COMPLETE_ROW_PROFILE
            )
            self.assertEqual(admitted["complete_row_bytes"], 64 * 1024**2)
            self.assertEqual(admitted["dtype_row_limit"], maximum)
            rows = np.lib.stride_tricks.as_strided(
                np.zeros(2, dtype), shape=(maximum + 1, 2), strides=(0, itemsize)
            )
            with (
                self.subTest(dtype=dtype),
                patch.object(
                    actions, "_snapshot", side_effect=AssertionError("copied")
                ),
                patch.object(
                    actions, "_sklearn", side_effect=AssertionError("imported")
                ),
                self.assertRaisesRegex(ValueError, "byte budget"),
            ):
                actions.fit_control_scaler(
                    rows,
                    source_id="byte-control",
                    row_profile=actions.COMPLETE_ROW_PROFILE,
                )
        common_rows = actions.COMPLETE_ROW_BYTES // 16 + 1
        actions._admit_row_shape((common_rows, 2), 4, actions.COMPLETE_ROW_PROFILE)
        with self.assertRaisesRegex(ValueError, "byte budget"):
            actions._admit_row_shape((common_rows, 2), 8, actions.COMPLETE_ROW_PROFILE)
        with self.assertRaisesRegex(ValueError, "byte budget"):
            actions._admit_row_shape((2**100, 2), 8, actions.COMPLETE_ROW_PROFILE)

    def test_invalid_fit_inputs_fail_before_optional_import(self):
        invalid = [
            np.zeros((1, 2), np.float32),
            np.zeros((2, 3), np.float32),
            np.zeros((2, 2), np.int32),
            np.zeros((2, 2), ">f4"),
            np.array([[0, 0], [np.nan, 1]], np.float32),
            np.array([[0, 0], [1, np.inf]], np.float32),
            np.array([[0, 0], [1, 1], [np.nan, np.inf]], np.float32),
        ]
        with patch.object(actions, "_sklearn", side_effect=AssertionError("imported")):
            for value in invalid:
                with self.subTest(value=value), self.assertRaises(ValueError):
                    actions.fit_control_scaler(value, source_id="invalid-control")

    def test_array_subclass_cannot_replace_owned_rows(self):
        class Replacement(np.ndarray):
            def tobytes(self, *args, **kwargs):
                raise AssertionError("Subclass encoder executed")

        with self.assertRaisesRegex(ValueError, "exact ndarray"):
            actions.fit_control_scaler(
                np.zeros((2, 2), np.float32).view(Replacement), source_id="subclass"
            )

    def test_fit_budget_checks_before_copy(self):
        rows = np.lib.stride_tricks.as_strided(
            np.zeros(2, np.float32),
            shape=(actions.MAX_FIT_ROWS + 1, 2),
            strides=(0, 4),
        )
        with patch.object(actions, "_snapshot", side_effect=AssertionError("copied")):
            with self.assertRaisesRegex(ValueError, "bounded shape"):
                actions.fit_control_scaler(rows, source_id="oversized")

    def test_no_public_asserted_scaler_authority(self):
        with self.assertRaises(TypeError):
            actions.FittedActionScaler()
        with self.assertRaises(TypeError):
            actions.SupportedPushTCandidates()
        forged = object.__new__(actions.FittedActionScaler)
        for value in (forged, {"claimed_verified": True}, {"sha256": "a" * 64}):
            with (
                self.subTest(value=type(value)),
                self.assertRaisesRegex(ValueError, "owner-issued"),
            ):
                actions.prepare_candidates(
                    value, ("a", "b"), np.zeros((2, 25, 2), np.float32)
                )

    def test_original_standardized_contract_still_denies_raw_commands(self):
        value = StandardizedCandidates(("a", "b"), np.zeros((1, 2, 5, 10), np.float32))
        with self.assertRaisesRegex(ValueError, "Raw execution is unsupported"):
            value.raw_commands()


class QualifiedSklearnControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        try:
            verify_runtime()
        except (ValueError, ModuleNotFoundError) as error:
            raise unittest.SkipTest(
                "Requires the exact optional LeWM runtime"
            ) from error

    def fit(self, rows=None):
        if rows is None:
            rows = np.array([[-1, -1], [0, 0], [1, 1]], np.float64)
        return actions.fit_control_scaler(rows, source_id="paired-arithmetic-control")

    def pool(self):
        return np.linspace(-0.5, 0.5, 100, dtype=np.float32).reshape(2, 25, 2)

    def test_explicit_byte_profile_preserves_fit_bytes_and_synthetic_scope(self):
        for dtype in (np.float32, np.float64):
            rows = np.array([[-2, 0], [np.nan, 9], [0, 0], [3, 0]], dtype)
            legacy = actions.fit_control_scaler(rows, source_id="profile-parity")
            larger = actions.fit_control_scaler(
                rows,
                source_id="profile-parity",
                row_profile=actions.COMPLETE_ROW_PROFILE,
            )
            record = larger.receipt()
            admission = record.pop("row_admission")
            self.assertEqual(record, legacy.receipt())
            self.assertEqual(admission["complete_row_bytes"], rows.nbytes)
            self.assertEqual(record["scope"], "synthetic_control_only")
            for name in ("mean", "variance", "scale"):
                self.assertEqual(
                    larger.statistics[name].tobytes(), legacy.statistics[name].tobytes()
                )
            self.assertEqual(larger.fit_rows.tobytes(), rows.tobytes())
            np.testing.assert_array_equal(
                larger.retained_row_mask, [True, False, True, True]
            )
            for array in (larger.fit_rows, larger.retained_row_mask):
                with self.assertRaises(ValueError):
                    array.setflags(write=True)
            larger_rows = rows.copy()
            larger_rows[1, 1] = np.inf
            with self.assertRaisesRegex(ValueError, "Infinite"):
                actions.fit_control_scaler(
                    larger_rows,
                    source_id="profile-parity",
                    row_profile=actions.COMPLETE_ROW_PROFILE,
                )

    def test_explicit_profile_admits_complete_rows_above_legacy_limit(self):
        rows = np.zeros((actions.MAX_FIT_ROWS + 1, 2), np.float32)
        rows[0] = [-2, 1]
        rows[-2] = [np.nan, 9]
        rows[-1] = [3, -1]
        with self.assertRaisesRegex(ValueError, "bounded shape"):
            actions.fit_control_scaler(rows, source_id="larger-control")
        fitted = actions.fit_control_scaler(
            rows, source_id="larger-control", row_profile=actions.COMPLETE_ROW_PROFILE
        )
        self.assertEqual(fitted.fit_rows.tobytes(), rows.tobytes())
        self.assertEqual(fitted.receipt()["n_samples_seen"], actions.MAX_FIT_ROWS)
        self.assertEqual(fitted.receipt()["excluded_nan_rows"], 1)
        self.assertEqual(fitted.receipt()["scope"], "synthetic_control_only")
        self.assertFalse(fitted.retained_row_mask[-2])

    def test_population_scale_constant_axis_and_nan_rows(self):
        rows = np.array([[-1, 0], [0, 0], [np.nan, 500], [1, 0]], np.float64)
        fitted = self.fit(rows)
        np.testing.assert_array_equal(fitted.statistics["mean"], [0, 0])
        np.testing.assert_allclose(fitted.statistics["variance"], [2 / 3, 0])
        np.testing.assert_allclose(fitted.statistics["scale"], [np.sqrt(2 / 3), 1])
        self.assertNotEqual(fitted.statistics["scale"][0], 1)  # sample std is one
        np.testing.assert_array_equal(
            fitted.retained_row_mask, [True, True, False, True]
        )
        self.assertEqual(fitted.receipt()["n_samples_seen"], 3)
        self.assertEqual(fitted.receipt()["excluded_nan_rows"], 1)
        self.assertEqual(fitted.receipt()["scope"], "synthetic_control_only")
        self.assertNotIn("torch", sys.modules)

    def test_near_constant_axis_preserves_sklearn_scale_rule(self):
        fitted = self.fit(np.array([[1, 0], [1, 0], [np.nextafter(1.0, 2.0), 0]]))
        self.assertGreater(fitted.statistics["variance"][0], 0)
        np.testing.assert_array_equal(fitted.statistics["scale"], [1, 1])

    def test_row_order_and_dtype_remain_distinct_source_identities(self):
        rows = np.array([[-1, 0], [0, 0], [1, 0]], np.float64)
        first = self.fit(rows)
        reversed_rows = self.fit(rows[::-1])
        narrowed = self.fit(rows.astype(np.float32))
        np.testing.assert_array_equal(
            first.statistics["mean"], reversed_rows.statistics["mean"]
        )
        self.assertNotEqual(first.sha256, reversed_rows.sha256)
        self.assertNotEqual(first.sha256, narrowed.sha256)

    def test_fitted_statistics_and_inspection_are_immutable(self):
        rows = np.array([[-1, 0], [0, 0], [1, 0]], np.float64)
        fitted = self.fit(rows)
        before = fitted.sha256
        rows[:] = 99
        for array in (*fitted.statistics.values(), fitted.retained_row_mask):
            with self.assertRaises(ValueError):
                array.setflags(write=True)
        report = fitted.receipt()
        report["scope"] = "dataset_verified"
        report["statistics"]["mean"]["sha256"] = "0" * 64
        self.assertEqual(fitted.sha256, before)
        with self.assertRaises(AttributeError):
            fitted._receipt = "replacement"
        with self.assertRaises(AttributeError):
            copy.copy(fitted)

    def test_dense_order_matches_two_coordinate_sklearn_operations(self):
        from sklearn.preprocessing import StandardScaler

        rows = np.array([[-0.8, -0.1], [0.2, 0.3], [0.6, 0.9]], np.float64)
        fitted = self.fit(rows)
        raw = self.pool()[:, ::-1, :]
        prepared = actions.prepare_candidates(fitted, ("reverse-a", "reverse-b"), raw)
        direct = StandardScaler().fit(rows)
        for candidate in range(2):
            for primitive in range(25):
                expected = direct.transform(raw[candidate, primitive][None, :])[0]
                b, r = divmod(primitive, 5)
                np.testing.assert_array_equal(
                    prepared.standardized.values[0, candidate, b, 2 * r : 2 * r + 2],
                    expected,
                )
        expected_execution = direct.inverse_transform(
            prepared.standardized.values.reshape(-1, 2)
        ).reshape(2, 25, 2)
        np.testing.assert_array_equal(prepared.executable, expected_execution)
        self.assertNotEqual(
            prepared.receipt()["standardized"]["sha256"],
            actions.prepare_candidates(
                fitted, ("reverse-a", "reverse-b"), raw[:, ::-1]
            ).receipt()["standardized"]["sha256"],
        )

    def test_legal_boundaries_and_just_outside_reject_without_clipping(self):
        fitted = self.fit()
        raw = np.empty((2, 25, 2), np.float32)
        raw[0] = -1
        raw[1] = 1
        prepared = actions.prepare_candidates(fitted, ("negative", "positive"), raw)
        self.assertTrue((np.abs(prepared.executable) <= 1).all())
        raw[1, 0, 0] = np.nextafter(np.float32(1), np.float32(2))
        with self.assertRaisesRegex(ValueError, "Raw proposals"):
            actions.prepare_candidates(fitted, ("negative", "positive"), raw)

    def test_float32_inverse_rounding_cannot_extend_support(self):
        # Frozen synthetic boundary: actual sklearn maps -1 to -1 - one float32 ULP.
        rows = np.array(
            [
                [-0.21676428616046906, -0.701422393321991],
                [-0.7363889813423157, 0.926403284072876],
                [-0.2295258790254593, -0.45703527331352234],
            ],
            np.float32,
        )
        fitted = self.fit(rows)
        raw = np.empty((2, 25, 2), np.float32)
        raw[0] = -1
        raw[1] = 1
        with self.assertRaisesRegex(ValueError, "Inverse-transformed"):
            actions.prepare_candidates(fitted, ("negative", "positive"), raw)
        raw *= 0.5
        prepared = actions.prepare_candidates(fitted, ("negative", "positive"), raw)
        self.assertGreater(prepared.receipt()["roundtrip_max_abs"], 0)
        self.assertTrue((np.abs(prepared.executable) <= 1).all())

    def test_candidate_copies_and_receipts_cannot_mutate_preparation(self):
        raw = self.pool()
        fitted = self.fit()
        prepared = actions.prepare_candidates(fitted, ("a", "b"), raw)
        before = prepared.sha256
        original = raw.copy()
        raw[:] = 0.8
        np.testing.assert_array_equal(prepared.proposed, original)
        for array in (
            prepared.proposed,
            prepared.executable,
            prepared.standardized.values,
        ):
            with self.assertRaises(ValueError):
                array.setflags(write=True)
        prepared.receipt()["candidate_ids"][0] = "changed"
        # Read-only ndarray storage still permits changes to view shape/dtype.
        # Each inspection must therefore return a fresh view or contract object.
        prepared.proposed.shape = (100,)
        prepared.executable.dtype = np.uint8
        exposed_standardized = prepared.standardized
        exposed_standardized.values.shape = (100,)
        self.assertEqual(prepared.proposed.shape, (2, 25, 2))
        self.assertEqual(prepared.executable.dtype, np.float32)
        self.assertEqual(prepared.standardized.values.shape, (1, 2, 5, 10))
        self.assertEqual(prepared.sha256, before)
        changed = actions.prepare_candidates(fitted, ("different", "b"), original)
        self.assertNotEqual(changed.sha256, before)
        self.assertEqual(prepared.receipt()["scaler_sha256"], fitted.sha256)

    def test_candidate_negative_controls_preserve_valid_counterpart(self):
        fitted = self.fit()
        for ids in (("a", "a"), ("a",), ("", "b"), ["a", "b"]):
            with self.subTest(ids=ids), self.assertRaises(ValueError):
                actions.prepare_candidates(fitted, ids, self.pool())
        for bad in (np.nan, np.inf, -np.inf, 1.01):
            raw = self.pool()
            raw[0, 0, 0] = bad
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                actions.prepare_candidates(fitted, ("a", "b"), raw)
        with self.assertRaises(ValueError):
            actions.prepare_candidates(
                fitted, ("a", "b"), self.pool().astype(np.float64)
            )
        self.assertEqual(
            actions.prepare_candidates(
                fitted, ("a", "b"), self.pool()
            ).standardized.ids,
            ("a", "b"),
        )

    def test_runtime_drift_does_not_reuse_a_previously_fitted_handle(self):
        fitted = self.fit()
        changed = fitted.receipt()["runtime_observation"]
        changed["profile_sha256"] = "0" * 64
        with patch.object(actions, "verify_runtime", return_value=changed):
            with self.assertRaisesRegex(ValueError, "runtime identity changed"):
                actions.prepare_candidates(fitted, ("a", "b"), self.pool())
        actions.prepare_candidates(fitted, ("a", "b"), self.pool())


if __name__ == "__main__":
    unittest.main()
