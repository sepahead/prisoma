"""Render the canonical sensor-execution guide as a deterministic vector PDF.

Publication dependencies stay separate from the application runtime.
The source uses headings, paragraphs, fenced text, one table, and native SVG.
"""

from __future__ import annotations

import argparse
import hashlib
import html
from importlib.metadata import version
from io import BytesIO
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "integrations/agent-bridge/GUIDE.md"
OUTPUT = ROOT / "output/pdf/Recorded_Sensor_Execution.pdf"
RECEIPT = ROOT / "integrations/agent-bridge/publication.json"


def identity(path: Path) -> dict:
    raw = path.read_bytes()
    return {"bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def render() -> tuple[bytes, dict]:
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
        Table,
        TableStyle,
    )
    from svglib.svglib import svg2rlg

    rl_config.invariant = 1
    font_root = Path("/System/Library/Fonts/Supplemental")
    fonts = {
        "Arial": font_root / "Arial.ttf",
        "Arial-Bold": font_root / "Arial Bold.ttf",
        "SensorMono": font_root / "Andale Mono.ttf",
    }
    for name, path in fonts.items():
        pdfmetrics.registerFont(TTFont(name, str(path)))
    pdfmetrics.registerFontFamily("Arial", normal="Arial", bold="Arial-Bold")
    navy, muted = colors.HexColor("#142c42"), colors.HexColor("#475e73")
    width = A4[0] - 92
    body = ParagraphStyle(
        "body",
        fontName="Arial",
        fontSize=10.3,
        leading=14.3,
        textColor=navy,
        spaceAfter=8,
    )
    title = ParagraphStyle(
        "title",
        parent=body,
        fontName="Arial-Bold",
        fontSize=26,
        leading=31,
        spaceAfter=18,
        keepWithNext=True,
    )
    heading = ParagraphStyle(
        "heading",
        parent=body,
        fontName="Arial-Bold",
        fontSize=17,
        leading=22,
        spaceBefore=8,
        spaceAfter=12,
        keepWithNext=True,
    )
    code_style = ParagraphStyle(
        "code",
        parent=body,
        fontName="SensorMono",
        fontSize=8.8,
        leading=12.5,
        backColor=colors.HexColor("#edf3f7"),
        borderPadding=9,
        spaceBefore=5,
        spaceAfter=14,
    )
    cell_style = ParagraphStyle("cell", parent=body, fontSize=9, leading=12)

    def inline(text: str) -> str:
        text = html.escape(text)

        def link(match):
            label, target = match.groups()
            if not target.startswith("https://"):
                relative = (SOURCE.parent / target).resolve().relative_to(ROOT)
                target = "https://github.com/sepahead/prisoma/blob/main/" + str(
                    relative
                )
            return f'<link href="{target}" color="#315da0">{label}</link>'

        text = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", link, text)
        return re.sub(r"`([^`]+)`", r'<font name="SensorMono">\1</font>', text)

    story = []
    figures = {}
    lines = SOURCE.read_text().splitlines()
    index = 0
    while index < len(lines):
        line = lines[index].strip()
        index += 1
        if not line:
            continue
        if line == "<!-- pagebreak -->":
            story.append(PageBreak())
        elif line.startswith("# "):
            story.append(Paragraph(inline(line[2:]), title))
        elif line.startswith("## "):
            story.append(Paragraph(inline(line[3:]), heading))
        elif line == "```text":
            block = []
            while index < len(lines) and lines[index] != "```":
                block.append(lines[index])
                index += 1
            if index == len(lines):
                raise ValueError("unclosed text fence")
            index += 1
            story.append(Preformatted("\n".join(block), code_style))
        elif line.startswith("!["):
            match = re.fullmatch(r"!\[([^\]]+)\]\(([^)]+)\)", line)
            if match is None:
                raise ValueError("unsupported image syntax")
            path = (SOURCE.parent / match[2]).resolve()
            figures[str(path.relative_to(ROOT))] = identity(path)
            drawing = svg2rlg(str(path))
            if drawing is None or not (
                0 < drawing.width < 4096 and 0 < drawing.height < 4096
            ):
                raise ValueError("invalid vector figure dimensions")
            scale = min(width / drawing.width, 450 / drawing.height)
            drawing.scale(scale, scale)
            drawing.width *= scale
            drawing.height *= scale
            story.extend((drawing, Spacer(1, 10)))
        elif line.startswith("|"):
            rows = [line]
            while index < len(lines) and lines[index].startswith("|"):
                rows.append(lines[index])
                index += 1
            cells = []
            for row in rows:
                if re.fullmatch(r"[| :\-]+", row):
                    continue
                cells.append(
                    [
                        Paragraph(inline(cell.strip()), cell_style)
                        for cell in row.strip("|").split("|")
                    ]
                )
            if any(len(row) != 5 for row in cells):
                raise ValueError("sensor timing table requires five columns")
            table = Table(
                cells,
                colWidths=[width * f for f in (0.13, 0.21, 0.21, 0.27, 0.18)],
                repeatRows=1,
            )
            table.setStyle(
                TableStyle(
                    [
                        ("BACKGROUND", (0, 0), (-1, 0), colors.HexColor("#e6eef4")),
                        (
                            "ROWBACKGROUNDS",
                            (0, 1),
                            (-1, -1),
                            [colors.white, colors.HexColor("#f5f8fb")],
                        ),
                        ("VALIGN", (0, 0), (-1, -1), "TOP"),
                        ("TOPPADDING", (0, 0), (-1, -1), 7),
                        ("BOTTOMPADDING", (0, 0), (-1, -1), 7),
                        ("LINEBELOW", (0, 0), (-1, 0), 0.6, colors.HexColor("#bfcedb")),
                    ]
                )
            )
            story.extend((table, Spacer(1, 12)))
        else:
            paragraph = [line]
            while index < len(lines) and lines[index].strip():
                paragraph.append(lines[index].strip())
                index += 1
            story.append(Paragraph(inline(" ".join(paragraph)), body))

    def footer(canvas, document):
        canvas.saveState()
        canvas.setStrokeColor(colors.HexColor("#d3dfe8"))
        canvas.line(46, 43, A4[0] - 46, 43)
        canvas.setFont("Arial", 8)
        canvas.setFillColor(muted)
        canvas.drawString(
            46, 29, "Prisoma | Recorded sensor execution | Engineering scope"
        )
        canvas.drawRightString(A4[0] - 46, 29, str(document.page))
        canvas.restoreState()

    output = BytesIO()
    document = SimpleDocTemplate(
        output,
        pagesize=A4,
        leftMargin=46,
        rightMargin=46,
        topMargin=42,
        bottomMargin=59,
        title="Recorded sensor execution",
        author="Prisoma contributors",
        creator="Prisoma document renderer",
        pageCompression=1,
    )
    document.build(story, onFirstPage=footer, onLaterPages=footer)
    raw = output.getvalue()
    receipt = {
        "schema": "prisoma.sensor_execution_publication.v1",
        "source": identity(SOURCE),
        "renderer": identity(Path(__file__)),
        "figures": figures,
        "fonts": {path.name: identity(path) for path in fonts.values()},
        "packages": {name: version(name) for name in ("reportlab", "svglib")},
        "pdf": {
            "bytes": len(raw),
            "sha256": hashlib.sha256(raw).hexdigest(),
            "pages": document.page,
        },
        "scientific_validation": False,
    }
    return raw, receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raw, receipt = render()
    if args.check:
        if OUTPUT.read_bytes() != raw or json.loads(RECEIPT.read_text()) != receipt:
            raise SystemExit("PDF or publication identity drift")
    else:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_bytes(raw)
        RECEIPT.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    print(json.dumps(receipt["pdf"], sort_keys=True))


if __name__ == "__main__":
    main()
