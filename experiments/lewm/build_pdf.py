"""Render the canonical LeWM mathematics note and native SVG as a vector PDF.

Optional publication dependencies: ReportLab and svglib. Neither enters inference.
"""

from __future__ import annotations

import argparse
import hashlib
import html
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build(output: Path) -> dict:
    from reportlab import rl_config
    from reportlab.lib import colors
    from reportlab.lib.pagesizes import A4
    from reportlab.lib.styles import ParagraphStyle
    from reportlab.pdfbase import pdfmetrics
    from reportlab.pdfbase.ttfonts import TTFont
    from reportlab.platypus import (
        PageBreak,
        Paragraph,
        Preformatted,
        SimpleDocTemplate,
        Spacer,
    )
    from svglib.svglib import svg2rlg

    source = ROOT / "docs/lewm/MATHEMATICS.md"
    diagram = ROOT / "docs/lewm/inference.svg"
    fonts = {
        "LeWMSans": Path("/System/Library/Fonts/Supplemental/Arial.ttf"),
        "LeWMBold": Path("/System/Library/Fonts/Supplemental/Arial Bold.ttf"),
        "LeWMMono": Path("/System/Library/Fonts/Supplemental/Andale Mono.ttf"),
    }
    for name, path in fonts.items():
        pdfmetrics.registerFont(TTFont(name, str(path)))
    rl_config.invariant = 1
    navy = colors.HexColor("#16283d")
    accent = colors.HexColor("#235aa6")
    styles = {
        "title": ParagraphStyle(
            "title",
            fontName="LeWMBold",
            fontSize=28,
            leading=34,
            textColor=navy,
            spaceAfter=15,
        ),
        "heading": ParagraphStyle(
            "heading",
            fontName="LeWMBold",
            fontSize=21,
            leading=27,
            textColor=navy,
            spaceAfter=17,
        ),
        "body": ParagraphStyle(
            "body",
            fontName="LeWMSans",
            fontSize=10.5,
            leading=15.5,
            textColor=navy,
            spaceAfter=10,
        ),
        "small": ParagraphStyle(
            "small",
            fontName="LeWMSans",
            fontSize=9,
            leading=13,
            textColor=navy,
            spaceAfter=8,
        ),
        "code": ParagraphStyle(
            "code",
            fontName="LeWMMono",
            fontSize=9.2,
            leading=13,
            leftIndent=10,
            rightIndent=10,
            backColor=colors.HexColor("#edf3fa"),
            borderPadding=10,
            spaceBefore=8,
            spaceAfter=16,
        ),
    }

    def inline(text):
        text = html.escape(text)
        return re.sub(r"`([^`]+)`", r'<font name="LeWMMono">\1</font>', text)

    story = [
        Spacer(1, 20),
        Paragraph("LeWorldModel<br/>From pixels to a scored action", styles["title"]),
        Paragraph("A Prisoma mathematics and evidence guide", styles["body"]),
        Paragraph(
            "The local engineering adapter computes pretrained forecasts and complete candidate searches. It executes no raw action and establishes no scientific result.",
            styles["body"],
        ),
        Spacer(1, 12),
    ]
    drawing = svg2rlg(str(diagram))
    scale = (A4[0] - 96) / drawing.width
    drawing.scale(scale, scale)
    drawing.width *= scale
    drawing.height *= scale
    story.extend(
        [
            drawing,
            Spacer(1, 18),
            Paragraph("Read the symbols before reading the score", styles["heading"]),
            Paragraph(
                "Pixels, arena coordinates, standardized actions, and learned latent vectors are different quantities. The following pages state their units and show each calculation with small examples.",
                styles["body"],
            ),
            Paragraph(
                "The SVG and PDF use vector geometry and text. Zoom to inspect the model path and its evidence boundary.",
                styles["small"],
            ),
            Paragraph(
                f"Canonical source: docs/lewm/MATHEMATICS.md<br/>Source SHA-256: {digest(source)}",
                styles["small"],
            ),
        ]
    )
    lines = source.read_text().splitlines()
    paragraph = []
    code = None

    def flush():
        if paragraph:
            story.append(Paragraph(inline(" ".join(paragraph)), styles["body"]))
            paragraph.clear()

    for line in lines:
        if line.startswith("# "):
            continue
        if line.startswith("## "):
            flush()
            story.extend(
                [
                    Spacer(1, 20) if line[3:] == "Goal score" else PageBreak(),
                    Paragraph(inline(line[3:]), styles["heading"]),
                ]
            )
        elif line.startswith("```"):
            flush()
            if code is None:
                code = []
            else:
                story.append(Preformatted("\n".join(code), styles["code"]))
                code = None
        elif code is not None:
            code.append(line)
        elif not line.strip():
            flush()
        elif re.match(r"^\d+\. ", line):
            flush()
            story.append(Paragraph(inline(line), styles["body"]))
        else:
            paragraph.append(line)
    flush()
    if code is not None:
        raise ValueError("Unclosed Markdown code fence")

    def decorate(canvas, _):
        canvas.saveState()
        canvas.setStrokeColor(accent)
        canvas.setLineWidth(1.5)
        canvas.line(48, A4[1] - 35, A4[0] - 48, A4[1] - 35)
        canvas.setFont("LeWMSans", 8)
        canvas.setFillColor(navy)
        canvas.drawString(48, A4[1] - 26, "PRISOMA  /  LEWM ENGINEERING")
        canvas.drawString(48, 26, "Derived publication view • M2 and W1-W3 remain open")
        canvas.drawRightString(A4[0] - 48, 26, str(canvas.getPageNumber()))
        canvas.restoreState()

    output.parent.mkdir(parents=True, exist_ok=True)
    doc = SimpleDocTemplate(
        str(output),
        pagesize=A4,
        leftMargin=48,
        rightMargin=48,
        topMargin=54,
        bottomMargin=48,
        title="LeWorldModel: mathematics and evidence",
        author="Prisoma",
    )
    doc.build(story, onFirstPage=decorate, onLaterPages=decorate)
    return {
        "schema": "prisoma.lewm.math-publication.v1",
        "source_sha256": digest(source),
        "svg_sha256": digest(diagram),
        "renderer_sha256": digest(Path(__file__)),
        "pdf_sha256": digest(output),
        "font_sha256": {name: digest(path) for name, path in fonts.items()},
        "source_authority": "Markdown",
        "visual_review": "required_separately",
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / "output/pdf/LeWM_Mathematics_and_Evidence.pdf",
    )
    args = parser.parse_args()
    result = build(args.output)
    receipt = args.output.with_suffix(".build.json")
    receipt.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(json.dumps(result, indent=2))
