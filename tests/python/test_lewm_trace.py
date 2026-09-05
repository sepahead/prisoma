"""Synthetic positive and corruption controls for the independent verifier."""

import unittest

import numpy as np

from experiments.lewm.verify import objective, verify_round


class TraceControls(unittest.TestCase):
    def setUp(self):
        rng = np.random.default_rng(42)
        candidates = rng.normal(size=(1, 300, 5, 10)).astype(np.float32)
        candidates[:, 0] = 0
        forecast = rng.normal(size=(1, 300, 6, 192)).astype(np.float32)
        goal = rng.normal(size=(1, 1, 192)).astype(np.float32)
        costs = ((forecast[..., -1, :] - goal) ** 2).sum(axis=-1)
        indexes = np.argsort(costs, axis=1)[:, :30]
        elite = candidates[0, indexes[0]][None]
        self.arrays = {
            "candidates": candidates,
            "forecast": forecast,
            "goal_embedding": goal,
            "costs": costs,
            "topk_inds": indexes,
            "topk_candidates": elite,
            "topk_vals": costs[0, indexes[0]][None],
            "mean": elite.mean(axis=1),
            "var": elite.std(axis=1, ddof=1),
            "prev_mean": np.zeros((1, 5, 10), dtype=np.float32),
            "prev_var": np.ones((1, 5, 10), dtype=np.float32),
        }

    def test_valid_synthetic_round(self):
        verify_round(self.arrays)

    def test_duplicate_elite_rejected(self):
        self.arrays["topk_inds"][0, 1] = self.arrays["topk_inds"][0, 0]
        with self.assertRaisesRegex(ValueError, "Elite membership"):
            verify_round(self.arrays)

    def test_population_standard_deviation_rejected(self):
        self.arrays["var"] = self.arrays["topk_candidates"].std(axis=1, ddof=0)
        with self.assertRaisesRegex(ValueError, "sample standard deviation"):
            verify_round(self.arrays)

    def test_changed_forecast_rejected(self):
        self.arrays["forecast"][0, 200, -1, 0] += 10
        with self.assertRaisesRegex(ValueError, "Forecast/goal/cost"):
            verify_round(self.arrays)

    def test_fabricated_cost_rejected(self):
        index = next(i for i in range(300) if i not in self.arrays["topk_inds"])
        self.arrays["costs"][0, index] += 10
        with self.assertRaisesRegex(ValueError, "Forecast/goal/cost"):
            verify_round(self.arrays)

    def test_nonfinite_rejected(self):
        self.arrays["forecast"][0, 0, 0, 0] = np.nan
        with self.assertRaisesRegex(ValueError, "Invalid array"):
            verify_round(self.arrays)

    def test_changed_initial_mean_rejected(self):
        self.arrays["candidates"][0, 0, 0, 0] = 1
        with self.assertRaises(ValueError):
            verify_round(self.arrays)

    def test_short_pool_rejected(self):
        self.arrays["candidates"] = self.arrays["candidates"][:, :-1]
        with self.assertRaisesRegex(ValueError, "Invalid shape: candidates"):
            verify_round(self.arrays)

    def test_substituted_goal_rejected_even_with_matching_local_costs(self):
        old_goal = self.arrays["goal_embedding"].copy()
        self.arrays["goal_embedding"] += 1
        costs = (
            (self.arrays["forecast"][..., -1, :] - self.arrays["goal_embedding"]) ** 2
        ).sum(axis=-1)
        indexes = np.argsort(costs, axis=1)[:, :30]
        elite = self.arrays["candidates"][0, indexes[0]][None]
        self.arrays.update(
            costs=costs,
            topk_inds=indexes,
            topk_candidates=elite,
            topk_vals=costs[0, indexes[0]][None],
            mean=elite.mean(axis=1),
            var=elite.std(axis=1, ddof=1),
        )
        verify_round(self.arrays)
        with self.assertRaisesRegex(ValueError, "Changed goal objective"):
            objective(self.arrays["goal_embedding"], old_goal)

    def test_broadcastable_goal_shape_rejected(self):
        with self.assertRaisesRegex(ValueError, "Goal embedding shape"):
            objective(self.arrays["goal_embedding"][0], self.arrays["goal_embedding"])

    def test_fixed_goal_accepted(self):
        objective(self.arrays["goal_embedding"], self.arrays["goal_embedding"].copy())


if __name__ == "__main__":
    unittest.main()
