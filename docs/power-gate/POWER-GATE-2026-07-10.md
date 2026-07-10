# §14.8.3 Power Gate — idealized endpoint-sensitivity analysis (H1–H4)

Replicates/cell: 400 · bootstrap: 500 · one-sided α = 0.05 · target power = 0.8

The grid counts below are idealized planning sensitivities, **not capture requirements or guarantees**. A count is selected only when its own same-n null cell meets the Monte-Carlo size tolerance. H2 and H4 use separate endpoint labels, DGP tags, seeds, and preregistered directions.

## Verdicts

| Endpoint | DGP tag | Direction | Unit | Min | Smallest passing grid n (not a requirement) | Same-n null rate / tolerance | Idealized pass |
|---|---|---|---|---|---|---|---|
| H1 incremental ΔAUROC | `h1_binormal_incremental_auroc_v1` | positive | episodes | 0.050 | NOT REACHED WITH VALID SAME-n NULL | n/a | ❌ |
| H2 Red vs ablation-slope Spearman rho | `h2_positive_marginal_spearman_family_outcome_re_v2` | positive | tasks | 0.300 | 64 | 0.050 / 0.083 | ✅ |
| H3 mean per-case Kendall tau | `h3_ordered_gaussian_score_noise_family_icc_v2` | positive | matched cases | 0.333 | 40 | 0.068 / 0.083 | ✅ |
| H4 SSI vs L0-to-L2 degradation Spearman rho | `h4_negative_marginal_spearman_family_outcome_re_v2` | negative | tasks | 0.300 | 96 | 0.068 / 0.083 | ✅ |

## H1 (episodes × ΔAUROC; includes the null column)

| n | effect | power (sig ∧ point≥min) | sig. rate | futility rate | mean point |
|---|---|---|---|---|---|
| 40 | 0.000 | 0.045 | 0.045 | 0.145 | -0.0164 |
| 40 | 0.030 | 0.075 | 0.075 | 0.158 | 0.0124 |
| 40 | 0.050 | 0.085 | 0.085 | 0.113 | 0.0320 |
| 40 | 0.080 | 0.145 | 0.145 | 0.065 | 0.0765 |
| 80 | 0.000 | 0.035 | 0.037 | 0.165 | -0.0161 |
| 80 | 0.030 | 0.105 | 0.117 | 0.113 | 0.0109 |
| 80 | 0.050 | 0.147 | 0.155 | 0.060 | 0.0409 |
| 80 | 0.080 | 0.217 | 0.225 | 0.030 | 0.0695 |
| 160 | 0.000 | 0.003 | 0.007 | 0.333 | -0.0139 |
| 160 | 0.030 | 0.158 | 0.185 | 0.070 | 0.0222 |
| 160 | 0.050 | 0.220 | 0.260 | 0.035 | 0.0453 |
| 160 | 0.080 | 0.422 | 0.438 | 0.005 | 0.0805 |
| 320 | 0.000 | 0.003 | 0.013 | 0.635 | -0.0092 |
| 320 | 0.030 | 0.145 | 0.250 | 0.048 | 0.0242 |
| 320 | 0.050 | 0.312 | 0.370 | 0.018 | 0.0446 |
| 320 | 0.080 | 0.595 | 0.615 | 0.000 | 0.0780 |
| 480 | 0.000 | 0.000 | 0.030 | 0.772 | -0.0055 |
| 480 | 0.030 | 0.128 | 0.343 | 0.045 | 0.0242 |
| 480 | 0.050 | 0.422 | 0.593 | 0.003 | 0.0484 |
| 480 | 0.080 | 0.703 | 0.728 | 0.000 | 0.0750 |
| 640 | 0.000 | 0.000 | 0.018 | 0.858 | -0.0041 |
| 640 | 0.030 | 0.125 | 0.438 | 0.020 | 0.0265 |
| 640 | 0.050 | 0.445 | 0.652 | 0.003 | 0.0475 |
| 640 | 0.080 | 0.795 | 0.835 | 0.000 | 0.0771 |
| 960 | 0.000 | 0.000 | 0.020 | 0.970 | -0.0034 |
| 960 | 0.030 | 0.090 | 0.522 | 0.007 | 0.0266 |
| 960 | 0.050 | 0.468 | 0.765 | 0.000 | 0.0477 |
| 960 | 0.080 | 0.897 | 0.948 | 0.000 | 0.0788 |

## H2 (tasks; predicted marginal Spearman rho > 0)

| n | effect | power | mean point |
|---|---|---|---|
| 8 | 0.300 | 0.450 | 0.3052 |
| 12 | 0.300 | 0.407 | 0.2758 |
| 16 | 0.300 | 0.320 | 0.2685 |
| 24 | 0.300 | 0.453 | 0.2960 |
| 32 | 0.300 | 0.520 | 0.2896 |
| 48 | 0.300 | 0.677 | 0.2933 |
| 64 | 0.300 | 0.805 | 0.2995 |
| 96 | 0.300 | 0.927 | 0.2977 |
| 128 | 0.300 | 0.950 | 0.2965 |

## H2 null (marginal rho = 0; positive-tail size check)

| n | effect | power | mean point |
|---|---|---|---|
| 8 | 0.000 | 0.210 | 0.0085 |
| 12 | 0.000 | 0.115 | -0.0232 |
| 16 | 0.000 | 0.098 | -0.0147 |
| 24 | 0.000 | 0.070 | -0.0025 |
| 32 | 0.000 | 0.072 | 0.0080 |
| 48 | 0.000 | 0.065 | -0.0017 |
| 64 | 0.000 | 0.050 | -0.0044 |
| 96 | 0.000 | 0.050 | -0.0088 |
| 128 | 0.000 | 0.058 | 0.0004 |

## H3 (matched cases at mean τ = 1/3)

| n | effect | power | mean point |
|---|---|---|---|
| 20 | 0.333 | 0.682 | 0.3305 |
| 30 | 0.333 | 0.818 | 0.3383 |
| 40 | 0.333 | 0.860 | 0.3315 |
| 60 | 0.333 | 0.960 | 0.3342 |

## H3 null (size check)

| n | effect | power | mean point |
|---|---|---|---|
| 20 | 0.000 | 0.085 | -0.0109 |
| 30 | 0.000 | 0.102 | 0.0056 |
| 40 | 0.000 | 0.068 | -0.0006 |
| 60 | 0.000 | 0.060 | 0.0003 |

## H4 (tasks; predicted marginal Spearman rho < 0)

| n | effect | power | mean point |
|---|---|---|---|
| 8 | -0.300 | 0.400 | -0.2868 |
| 12 | -0.300 | 0.415 | -0.2876 |
| 16 | -0.300 | 0.393 | -0.2866 |
| 24 | -0.300 | 0.458 | -0.3057 |
| 32 | -0.300 | 0.507 | -0.2801 |
| 48 | -0.300 | 0.670 | -0.2889 |
| 64 | -0.300 | 0.757 | -0.2947 |
| 96 | -0.300 | 0.900 | -0.2909 |
| 128 | -0.300 | 0.983 | -0.2984 |

## H4 null (marginal rho = 0; negative-tail size check)

| n | effect | power | mean point |
|---|---|---|---|
| 8 | 0.000 | 0.207 | 0.0091 |
| 12 | 0.000 | 0.110 | 0.0303 |
| 16 | 0.000 | 0.098 | -0.0062 |
| 24 | 0.000 | 0.080 | 0.0102 |
| 32 | 0.000 | 0.058 | -0.0041 |
| 48 | 0.000 | 0.068 | 0.0042 |
| 64 | 0.000 | 0.055 | -0.0056 |
| 96 | 0.000 | 0.068 | -0.0068 |
| 128 | 0.000 | 0.062 | -0.0075 |

## Machine-readable limitations

- `grid_counts_not_capture_requirements`: Selected n values are the smallest passing points on finite idealized grids; they are not capture requirements or guarantees.
- `idealized_endpoint_dgps`: H2/H4 use calibrated Gaussian-copula endpoint pairs and H3 uses ordered Gaussian score noise; real endpoint measurement error and estimator instability are not simulated.
- `no_nested_capture_allocation`: The simulator has no family→task/case→episode→severity/window allocation, binomial outcomes, instruction-eligibility gate, or fitted-transform uncertainty; H2 and H4 still share one idealized copula family rather than endpoint-specific capture DGPs.
- `coarse_monte_carlo_size_tolerance`: The same-n size screen uses alpha plus three binomial Monte-Carlo standard errors. It is a transparent simulation tolerance, not evidence that a real test is calibrated exactly at nominal alpha.
- `h1_feature_path_not_implemented`: The H1 binormal feature model does not supply the train-reference local PID/CI scores, leakage tests, censoring rules, or missing mandatory baselines required by the scientific endpoint.
- `pilot_dependence_calibration_required`: Family sizes, H2/H4 outcome random-effect SD, and H3 latent score-error ICC require pilot justification before capture planning.

**Idealized sensitivity gate: NOT PASSED. Capture readiness: NOT ESTABLISHED.**
