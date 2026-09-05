"""Ordinary imports and the explicit, reviewed LeWM architecture."""

from __future__ import annotations

import importlib
import os
from pathlib import Path
import sys

from .assets import decode_verified_snapshot, projection, sha, verify_stage

ARMS = ("repository_jepa", "model_config_wheel_lewm")


class PreparedLeWM:
    """An actual model loaded from the admitted source bundle and checkpoint.

    This Python object is a trusted local library seam, not a malicious-code sandbox.
    It predicts standardized candidates and has no command or outcome operation.
    """

    def __init__(self, staged: Path, arm: str, device: str):
        if device not in ("cpu", "mps") or arm not in ARMS:
            raise ValueError("Unsupported device or source arm")
        if os.environ.get("PYTORCH_ENABLE_MPS_FALLBACK") != "0":
            raise ValueError("Set PYTORCH_ENABLE_MPS_FALLBACK=0 before importing Torch")
        from .assets import verify_runtime

        self.runtime = verify_runtime()
        self._source_identity = tuple(source_identity(staged, arm).items())
        self._model = construct(arm, owners(staged), staged).to(device)
        self.device = device

    @property
    def source(self) -> dict:
        """Return an inspection copy of the prepared owner's immutable identity."""
        return dict(self._source_identity)

    def forecast(self, observation, candidates, output: Path):
        from .contracts import _forecast_candidates

        return _forecast_candidates(
            self._model, observation, candidates, self.source, self.device, output
        )


def owners(staged: Path) -> dict:
    """Import real staged packages after verifying every projected source file."""
    verify_stage(staged)
    root = (staged / "packages").resolve()
    for name, module in tuple(sys.modules.items()):
        if name.split(".")[0] in ("stable_worldmodel", "prisoma_lewm_reference"):
            path = getattr(module, "__file__", None)
            if path is None or not Path(path).resolve().is_relative_to(root):
                raise ValueError("Foreign preloaded upstream package")
    sys.dont_write_bytecode = True
    if str(root) not in sys.path:
        sys.path.insert(0, str(root))
    # These are ordinary file-backed Python packages. No sys.modules entries are synthesized.
    importlib.import_module("stable_worldmodel.utils")
    importlib.import_module("stable_worldmodel.spaces")
    repository = importlib.import_module("prisoma_lewm_reference.jepa")
    repository_module = importlib.import_module("prisoma_lewm_reference.module")
    wheel = importlib.import_module("stable_worldmodel.wm.lewm.lewm")
    wheel_module = importlib.import_module("stable_worldmodel.wm.lewm.module")
    return {
        "repository_jepa": (repository.JEPA, repository_module, "ARPredictor"),
        "model_config_wheel_lewm": (wheel.LeWM, wheel_module, "Predictor"),
    }


def backbone():
    """Construct the pinned tiny ViT directly, without a pretraining-package import.

    This is the pretrained=False branch of stable-pretraining 0.1.7 vit_hf:
    tiny, patch14, image224, no pooling, no mask token. Its MIT notice is staged.
    No constructor fetches weights. Strict state loading follows construction.
    """
    from transformers import ViTConfig, ViTModel

    config = ViTConfig(
        hidden_size=192,
        num_hidden_layers=12,
        num_attention_heads=3,
        intermediate_size=768,
        image_size=224,
        patch_size=14,
    )
    model = ViTModel(config, add_pooling_layer=False, use_mask_token=False)
    model.config.interpolate_pos_encoding = True
    return model


def construct(arm: str, constructors: dict, staged: Path):
    if arm not in ARMS:
        raise ValueError("Unknown model source arm")
    import torch

    owner, module, predictor = constructors[arm]
    model = owner(
        encoder=backbone(),
        predictor=getattr(module, predictor)(
            num_frames=3,
            input_dim=192,
            hidden_dim=192,
            output_dim=192,
            depth=6,
            heads=16,
            mlp_dim=2048,
            dim_head=64,
            dropout=0.1,
            emb_dropout=0.0,
        ),
        action_encoder=module.Embedder(input_dim=10, emb_dim=192),
        projector=module.MLP(
            input_dim=192, output_dim=192, hidden_dim=2048, norm_fn=torch.nn.BatchNorm1d
        ),
        pred_proj=module.MLP(
            input_dim=192, output_dim=192, hidden_dim=2048, norm_fn=torch.nn.BatchNorm1d
        ),
    )
    expected = projection()["assets"]["model/weights.pt"]
    path = staged / "assets/model/weights.pt"
    state = decode_verified_snapshot(
        path,
        expected,
        lambda stream: torch.load(stream, map_location="cpu", weights_only=True),
    )
    if (
        not isinstance(state, dict)
        or not state
        or not all(
            isinstance(key, str)
            and isinstance(value, torch.Tensor)
            and torch.isfinite(value).all()
            for key, value in state.items()
        )
    ):
        raise ValueError("Checkpoint requires a finite tensor state dictionary")
    model.load_state_dict(state, strict=True)
    model.eval().requires_grad_(False)
    return model


def preprocess(current, goal):
    from torchvision.transforms import v2
    import torch

    transform = v2.Compose(
        [
            v2.ToImage(),
            v2.ToDtype(torch.float32, scale=True),
            v2.Normalize(mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]),
            v2.Resize(size=224),
        ]
    )
    return transform(current), transform(goal)


def cost_call(model, pixels, goal, actions, device):
    import torch

    samples = actions.shape[1]
    info = {
        "pixels": pixels[None, None, None].expand(1, samples, 1, -1, -1, -1).to(device),
        "goal": goal[None, None, None].expand(1, samples, 1, -1, -1, -1).to(device),
        "action": torch.zeros(1, samples, 1, 10, device=device),
    }
    with torch.inference_mode():
        costs = model.get_cost(info, actions.to(device))
    return tuple(
        value.detach().cpu().numpy()
        for value in (costs, info["predicted_emb"], info["goal_emb"])
    )


def source_identity(staged: Path, arm: str) -> dict:
    if arm not in ARMS:
        raise ValueError("Unknown model source arm")
    return {
        "arm": arm,
        "projection_sha256": sha(Path(__file__).with_name("projection.json")),
        "source_manifest_sha256": sha(staged / "source-manifest.json"),
        "constructor_sha256": sha(Path(__file__)),
        "checkpoint_sha256": projection()["assets"]["model/weights.pt"]["sha256"],
    }
