#!/usr/bin/env python3
"""Render the canonical PID method contract as a deterministic PDF.

The Markdown source remains authoritative. This renderer creates a publication view
with a cover, table of contents, repeated table headings, vector text, and rendered
display equations. It does not interpret or revise the scientific content.
"""

from __future__ import annotations

import argparse
import hashlib
import html
import json
import re
import shutil
import sys
from dataclasses import dataclass
from pathlib import Path

import matplotlib

matplotlib.use("Agg")

from matplotlib.font_manager import FontProperties
from matplotlib.mathtext import math_to_image
from reportlab import rl_config
from reportlab.lib import colors
from reportlab.lib.enums import TA_LEFT
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import mm
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.platypus import (
    BaseDocTemplate,
    Frame,
    Image,
    KeepTogether,
    ListFlowable,
    ListItem,
    PageBreak,
    PageTemplate,
    Paragraph,
    Preformatted,
    Spacer,
    Table,
    TableStyle,
)
from reportlab.platypus.tableofcontents import TableOfContents

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE = ROOT / "PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md"
DEFAULT_OUTPUT = ROOT / "output/pdf/PID_Method_Selection_and_Publication_Contract.pdf"
DEFAULT_WORK_DIR = ROOT / "tmp/pdfs/pid-method-contract"

MAX_SOURCE_BYTES = 2 * 1024 * 1024
MAX_TABLE_ROWS = 512
MAX_TABLE_COLUMNS = 8
PAGE_WIDTH, PAGE_HEIGHT = A4
LEFT_MARGIN = 18 * mm
RIGHT_MARGIN = 18 * mm
TOP_MARGIN = 19 * mm
BOTTOM_MARGIN = 18 * mm

NAVY = colors.HexColor("#102A43")
BLUE = colors.HexColor("#1769AA")
CYAN = colors.HexColor("#2CB1BC")
PALE_BLUE = colors.HexColor("#EAF4FB")
PALE_CYAN = colors.HexColor("#E8F8F7")
PALE_GRAY = colors.HexColor("#F4F7FA")
MID_GRAY = colors.HexColor("#627D98")
LIGHT_GRAY = colors.HexColor("#D9E2EC")
TEXT = colors.HexColor("#243B53")
WHITE = colors.white

FONT_REGULAR_PATH = Path("/System/Library/Fonts/Supplemental/Arial.ttf")
FONT_BOLD_PATH = Path("/System/Library/Fonts/Supplemental/Arial Bold.ttf")
FONT_ITALIC_PATH = Path("/System/Library/Fonts/Supplemental/Arial Italic.ttf")
FONT_BOLD_ITALIC_PATH = Path("/System/Library/Fonts/Supplemental/Arial Bold Italic.ttf")
FONT_MONO_PATH = Path("/System/Library/Fonts/Supplemental/Andale Mono.ttf")

STRUCTURAL_RE = re.compile(r"^(?:#{1,4}\s+|```|\\\[\s*$|\s*[-*]\s+|\s*\d+\.\s+|\s*\|)")
LIST_RE = re.compile(r"^(?P<indent>\s*)(?P<marker>[-*]|\d+\.)\s+(?P<body>.+)$")
TABLE_SEPARATOR_RE = re.compile(r"^\s*\|?\s*:?-{3,}:?\s*(?:\|\s*:?-{3,}:?\s*)+\|?\s*$")
LINK_RE = re.compile(r"\[([^\]]+)\]\(([^)]+)\)")
INLINE_CODE_RE = re.compile(r"`([^`]+)`")
INLINE_MATH_RE = re.compile(r"\\\((.+?)\\\)")


class RenderError(RuntimeError):
    """The source cannot be rendered under the bounded publication contract."""


@dataclass(frozen=True)
class RenderSummary:
    source_sha256: str
    source_bytes: int
    renderer_sha256: str
    renderer_bytes: int
    pdf_sha256: str
    pdf_bytes: int
    equation_count: int
    table_count: int
    heading_count: int


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def read_bounded_utf8(path: Path, limit: int) -> tuple[bytes, str]:
    payload = path.read_bytes()
    if len(payload) > limit:
        raise RenderError(f"source exceeds {limit} bytes: {path}")
    try:
        return payload, payload.decode("utf-8")
    except UnicodeDecodeError as error:
        raise RenderError(f"source is not UTF-8: {path}") from error


def register_fonts() -> None:
    font_paths = (
        FONT_REGULAR_PATH,
        FONT_BOLD_PATH,
        FONT_ITALIC_PATH,
        FONT_BOLD_ITALIC_PATH,
        FONT_MONO_PATH,
    )
    missing = [str(path) for path in font_paths if not path.is_file()]
    if missing:
        raise RenderError(f"required fonts are absent: {missing}")
    pdfmetrics.registerFont(TTFont("PrisomaSans", str(FONT_REGULAR_PATH)))
    pdfmetrics.registerFont(TTFont("PrisomaSans-Bold", str(FONT_BOLD_PATH)))
    pdfmetrics.registerFont(TTFont("PrisomaSans-Italic", str(FONT_ITALIC_PATH)))
    pdfmetrics.registerFont(
        TTFont("PrisomaSans-BoldItalic", str(FONT_BOLD_ITALIC_PATH))
    )
    pdfmetrics.registerFont(TTFont("PrisomaMono", str(FONT_MONO_PATH)))
    pdfmetrics.registerFontFamily(
        "PrisomaSans",
        normal="PrisomaSans",
        bold="PrisomaSans-Bold",
        italic="PrisomaSans-Italic",
        boldItalic="PrisomaSans-BoldItalic",
    )


def latex_to_plain(value: str) -> str:
    replacements = (
        (r"\widetilde{UI}", "UĨ"),
        (r"\widehat{\delta}", "δ̂"),
        (r"\mathbb{E}", "E"),
        (r"\mathbb E", "E"),
        (r"\mathrm{KL}", "KL"),
        (r"\mathcal G", "G"),
        (r"\mathcal{G}", "G"),
        (r"\lambda", "λ"),
        (r"\delta", "δ"),
        (r"\Delta", "Δ"),
        (r"\sim", "~"),
        (r"\geq", "≥"),
        (r"\leq", "≤"),
        (r"\infty", "∞"),
        (r"\downarrow", "↓"),
        (r"\rightarrow", "→"),
        (r"\to", "→"),
        (r"\setminus", " without "),
        (r"\cap", " intersection "),
        (r"\cup", " union "),
        (r"\mid", " | "),
        (r"\Vert", "||"),
        (r"\circ", " o "),
        (r"\inf", "inf"),
        (r"\sup", "sup"),
        (r"\min", "min"),
        (r"\qquad", "   "),
        (r"\,", ""),
        (r"\;", " "),
        (r"\!", ""),
        (r"\left", ""),
        (r"\right", ""),
        (r"\bigl", ""),
        (r"\bigr", ""),
    )
    output = value
    for old, new in replacements:
        output = output.replace(old, new)
    output = re.sub(r"\\text\{([^{}]*)\}", r"\1", output)
    output = re.sub(r"\\mathbb\{([^{}]*)\}", r"\1", output)
    output = re.sub(r"\\mathrm\{([^{}]*)\}", r"\1", output)
    output = output.replace("{", "").replace("}", "")
    return re.sub(r"\s+", " ", output).strip()


def inline_markup(
    value: str,
    *,
    code_color: str = "#334E68",
    math_color: str = "#102A43",
    link_color: str = "#1769AA",
) -> str:
    placeholders: dict[str, str] = {}

    def store(fragment: str) -> str:
        key = f"@@PRISOMA{len(placeholders)}@@"
        placeholders[key] = fragment
        return key

    def code_replace(match: re.Match[str]) -> str:
        content = html.escape(match.group(1), quote=True)
        return store(
            f'<font name="PrisomaMono" color="{code_color}" size="8">{content}</font>'
        )

    def math_replace(match: re.Match[str]) -> str:
        content = html.escape(latex_to_plain(match.group(1)), quote=True)
        return store(f'<font color="{math_color}">{content}</font>')

    def link_replace(match: re.Match[str]) -> str:
        label = html.escape(match.group(1), quote=True)
        target = html.escape(match.group(2), quote=True)
        return store(f'<link href="{target}" color="{link_color}">{label}</link>')

    value = INLINE_CODE_RE.sub(code_replace, value)
    value = INLINE_MATH_RE.sub(math_replace, value)
    value = LINK_RE.sub(link_replace, value)
    value = html.escape(value, quote=True)
    value = re.sub(r"\*\*([^*]+)\*\*", r"<b>\1</b>", value)
    value = re.sub(r"(?<!\*)\*([^*]+)\*(?!\*)", r"<i>\1</i>", value)
    # Restore outer markup before the inner placeholders it may contain. For
    # example, a Markdown link label can itself contain inline code.
    for key, fragment in reversed(placeholders.items()):
        value = value.replace(key, fragment)
    return value


def split_table_row(line: str) -> list[str]:
    stripped = line.strip()
    if stripped.startswith("|"):
        stripped = stripped[1:]
    if stripped.endswith("|"):
        stripped = stripped[:-1]
    return [cell.strip() for cell in stripped.split("|")]


def build_styles() -> dict[str, ParagraphStyle]:
    sample = getSampleStyleSheet()
    return {
        "cover_title": ParagraphStyle(
            "CoverTitle",
            parent=sample["Title"],
            fontName="PrisomaSans-Bold",
            fontSize=28,
            leading=32,
            textColor=NAVY,
            alignment=TA_LEFT,
            spaceAfter=10,
        ),
        "cover_deck": ParagraphStyle(
            "CoverDeck",
            parent=sample["Normal"],
            fontName="PrisomaSans",
            fontSize=12,
            leading=17,
            textColor=MID_GRAY,
            spaceAfter=18,
        ),
        "cover_callout": ParagraphStyle(
            "CoverCallout",
            parent=sample["Normal"],
            fontName="PrisomaSans-Bold",
            fontSize=13,
            leading=18,
            textColor=WHITE,
            alignment=TA_LEFT,
        ),
        "h1": ParagraphStyle(
            "H1",
            parent=sample["Heading1"],
            fontName="PrisomaSans-Bold",
            fontSize=18,
            leading=22,
            textColor=NAVY,
            spaceBefore=14,
            spaceAfter=7,
            keepWithNext=True,
        ),
        "h2": ParagraphStyle(
            "H2",
            parent=sample["Heading2"],
            fontName="PrisomaSans-Bold",
            fontSize=14,
            leading=18,
            textColor=BLUE,
            spaceBefore=12,
            spaceAfter=6,
            keepWithNext=True,
        ),
        "h3": ParagraphStyle(
            "H3",
            parent=sample["Heading3"],
            fontName="PrisomaSans-Bold",
            fontSize=11.5,
            leading=15,
            textColor=NAVY,
            spaceBefore=9,
            spaceAfter=4,
            keepWithNext=True,
        ),
        "h4": ParagraphStyle(
            "H4",
            parent=sample["Heading4"],
            fontName="PrisomaSans-BoldItalic",
            fontSize=10,
            leading=13,
            textColor=BLUE,
            spaceBefore=7,
            spaceAfter=3,
            keepWithNext=True,
        ),
        "body": ParagraphStyle(
            "Body",
            parent=sample["BodyText"],
            fontName="PrisomaSans",
            fontSize=9.1,
            leading=13,
            textColor=TEXT,
            alignment=TA_LEFT,
            spaceAfter=6,
        ),
        "bullet": ParagraphStyle(
            "Bullet",
            parent=sample["BodyText"],
            fontName="PrisomaSans",
            fontSize=8.8,
            leading=12.4,
            textColor=TEXT,
        ),
        "reference": ParagraphStyle(
            "Reference",
            parent=sample["BodyText"],
            fontName="PrisomaSans",
            fontSize=7.8,
            leading=8.0,
            textColor=TEXT,
        ),
        "table": ParagraphStyle(
            "TableCell",
            parent=sample["BodyText"],
            fontName="PrisomaSans",
            fontSize=6.8,
            leading=8.6,
            textColor=TEXT,
        ),
        "table_header": ParagraphStyle(
            "TableHeader",
            parent=sample["BodyText"],
            fontName="PrisomaSans-Bold",
            fontSize=7,
            leading=8.8,
            textColor=WHITE,
        ),
        "code": ParagraphStyle(
            "Code",
            parent=sample["Code"],
            fontName="PrisomaMono",
            fontSize=7.1,
            leading=9.7,
            textColor=colors.HexColor("#243B53"),
            leftIndent=6,
            rightIndent=6,
            spaceBefore=4,
            spaceAfter=7,
        ),
        "quote": ParagraphStyle(
            "Quote",
            parent=sample["BodyText"],
            fontName="PrisomaSans-Italic",
            fontSize=9,
            leading=13,
            leftIndent=12,
            rightIndent=8,
            borderColor=CYAN,
            borderWidth=0,
            borderPadding=6,
            textColor=TEXT,
            backColor=PALE_CYAN,
            spaceAfter=7,
        ),
        "toc_title": ParagraphStyle(
            "TocTitle",
            parent=sample["Heading1"],
            fontName="PrisomaSans-Bold",
            fontSize=18,
            leading=22,
            textColor=NAVY,
            spaceAfter=12,
        ),
        "small": ParagraphStyle(
            "Small",
            parent=sample["BodyText"],
            fontName="PrisomaSans",
            fontSize=7.5,
            leading=10,
            textColor=MID_GRAY,
        ),
    }


class ContractDocTemplate(BaseDocTemplate):
    """Two-template document with heading bookmarks and a generated TOC."""

    def __init__(self, filename: str, *, styles: dict[str, ParagraphStyle]) -> None:
        super().__init__(
            filename,
            pagesize=A4,
            leftMargin=LEFT_MARGIN,
            rightMargin=RIGHT_MARGIN,
            topMargin=TOP_MARGIN,
            bottomMargin=BOTTOM_MARGIN,
            title="PID method-selection, mathematics, and publication contract",
            author="Sepehr Mahmoudian",
            subject="Prisoma research-governance and publication process",
            creator="Prisoma deterministic ReportLab renderer",
        )
        self.styles = styles
        cover_frame = Frame(
            17 * mm,
            17 * mm,
            PAGE_WIDTH - 34 * mm,
            PAGE_HEIGHT - 34 * mm,
            id="cover-frame",
            leftPadding=0,
            rightPadding=0,
            topPadding=0,
            bottomPadding=0,
        )
        body_frame = Frame(
            LEFT_MARGIN,
            BOTTOM_MARGIN,
            PAGE_WIDTH - LEFT_MARGIN - RIGHT_MARGIN,
            PAGE_HEIGHT - TOP_MARGIN - BOTTOM_MARGIN,
            id="body-frame",
            leftPadding=0,
            rightPadding=0,
            topPadding=0,
            bottomPadding=0,
        )
        self.addPageTemplates(
            [
                PageTemplate(
                    id="Cover",
                    frames=[cover_frame],
                    onPage=self._draw_cover_page,
                    autoNextPageTemplate="Body",
                ),
                PageTemplate(
                    id="Body",
                    frames=[body_frame],
                    onPage=self._prepare_body_page,
                    onPageEnd=self._draw_body_page,
                ),
            ]
        )

    @staticmethod
    def _set_metadata(canvas: object) -> None:
        canvas.setTitle("PID method-selection, mathematics, and publication contract")
        canvas.setAuthor("Sepehr Mahmoudian")
        canvas.setSubject("Prisoma research-governance and publication process")
        canvas.setCreator("Prisoma deterministic ReportLab renderer")
        canvas.setKeywords(
            "PID, Prisoma, method selection, reproducibility, publication"
        )

    def _draw_cover_page(self, canvas: object, _doc: object) -> None:
        self._set_metadata(canvas)
        canvas.saveState()
        canvas.setFillColor(NAVY)
        canvas.rect(0, PAGE_HEIGHT - 12 * mm, PAGE_WIDTH, 12 * mm, fill=1, stroke=0)
        canvas.setFillColor(CYAN)
        canvas.rect(0, 0, PAGE_WIDTH, 5 * mm, fill=1, stroke=0)
        canvas.setFillColor(MID_GRAY)
        canvas.setFont("PrisomaSans", 7.5)
        canvas.drawString(17 * mm, 10 * mm, "PRISOMA • RESEARCH-GOVERNANCE CONTRACT")
        canvas.restoreState()

    def _prepare_body_page(self, canvas: object, _doc: object) -> None:
        self._set_metadata(canvas)
        # Retain one clean page graphics state. Split tables and paragraphs may
        # leave a clipping path active even when their Python-level save/restore
        # calls are balanced; restore this baseline before drawing furniture.
        self._body_canvas_state_depth = len(canvas.state_stack)
        canvas.saveState()

    def _draw_body_page(self, canvas: object, _doc: object) -> None:
        target_depth = self._body_canvas_state_depth
        if len(canvas.state_stack) < target_depth:
            raise RenderError("page graphics-state stack underflow")
        while len(canvas.state_stack) > target_depth:
            canvas.restoreState()
        canvas.saveState()
        canvas.setFillColor(MID_GRAY)
        canvas.setFont("PrisomaSans", 7.2)
        canvas.drawRightString(
            PAGE_WIDTH - RIGHT_MARGIN,
            8.2 * mm,
            f"Page {canvas.getPageNumber()}",
        )
        canvas.restoreState()

    def afterFlowable(self, flowable: object) -> None:
        if not isinstance(flowable, Paragraph):
            return
        # The document-level H1 is rendered on the cover, so Markdown H2 is the
        # highest level in the body outline and generated table of contents.
        level_by_style = {"H1": 0, "H2": 0, "H3": 1, "H4": 2}
        level = level_by_style.get(flowable.style.name)
        if level is None:
            return
        text = flowable.getPlainText()
        key = getattr(flowable, "bookmark_name", f"heading-{self.seq.nextf('heading')}")
        self.canv.bookmarkPage(key)
        self.canv.addOutlineEntry(text, key, level=level, closed=level > 1)
        self.notify("TOCEntry", (level, text, self.page, key))


def cover_story(styles: dict[str, ParagraphStyle]) -> list[object]:
    status_rows = [
        [
            "Status",
            "Current research-governance contract; not a preregistration or result",
        ],
        ["Implementation pin", "pid-rs@796c11e70f009634b853dc4ada6f565563d82f51"],
        ["Review", "16 August 2026 • literature cutoff 13 August 2026"],
        ["Authority", "Canonical Markdown; this PDF is a deterministic derived view"],
    ]
    status_table = Table(
        [
            [
                Paragraph(f"<b>{html.escape(label)}</b>", styles["small"]),
                Paragraph(html.escape(value), styles["small"]),
            ]
            for label, value in status_rows
        ],
        colWidths=[34 * mm, 124 * mm],
    )
    status_table.setStyle(
        TableStyle(
            [
                ("BACKGROUND", (0, 0), (0, -1), PALE_BLUE),
                ("BACKGROUND", (1, 0), (1, -1), PALE_GRAY),
                ("BOX", (0, 0), (-1, -1), 0.5, LIGHT_GRAY),
                ("INNERGRID", (0, 0), (-1, -1), 0.35, LIGHT_GRAY),
                ("VALIGN", (0, 0), (-1, -1), "TOP"),
                ("LEFTPADDING", (0, 0), (-1, -1), 7),
                ("RIGHTPADDING", (0, 0), (-1, -1), 7),
                ("TOPPADDING", (0, 0), (-1, -1), 6),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 6),
            ]
        )
    )
    callout = Table(
        [
            [
                Paragraph(
                    "There is no generic PID result. A shared label, author, or software route "
                    "does not make two functionals, quantities, or estimands identical.",
                    styles["cover_callout"],
                )
            ]
        ],
        colWidths=[158 * mm],
    )
    callout.setStyle(
        TableStyle(
            [
                ("BACKGROUND", (0, 0), (-1, -1), NAVY),
                ("BOX", (0, 0), (-1, -1), 0, NAVY),
                ("LEFTPADDING", (0, 0), (-1, -1), 12),
                ("RIGHTPADDING", (0, 0), (-1, -1), 12),
                ("TOPPADDING", (0, 0), (-1, -1), 12),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 12),
            ]
        )
    )
    process = Table(
        [
            [
                Paragraph(
                    f"<b>{stage}</b><br/><font size='6.8'>{label}</font>",
                    styles["small"],
                )
                for stage, label in (
                    ("M0", "Preserve"),
                    ("M1", "Type"),
                    ("M2", "Prove"),
                    ("M3", "Admit"),
                    ("M4", "Design"),
                    ("M5", "Freeze"),
                    ("M6", "Execute"),
                    ("M7", "Challenge"),
                    ("M8", "Publish"),
                )
            ]
        ],
        colWidths=[158 * mm / 9] * 9,
    )
    process.setStyle(
        TableStyle(
            [
                ("BACKGROUND", (0, 0), (-1, -1), PALE_CYAN),
                ("BOX", (0, 0), (-1, -1), 0.5, CYAN),
                ("INNERGRID", (0, 0), (-1, -1), 0.35, colors.HexColor("#9CDAD7")),
                ("ALIGN", (0, 0), (-1, -1), "CENTER"),
                ("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
                ("TOPPADDING", (0, 0), (-1, -1), 6),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 6),
            ]
        )
    )
    return [
        Spacer(1, 27 * mm),
        Paragraph(
            "PID method-selection,<br/>mathematics, and<br/>publication contract",
            styles["cover_title"],
        ),
        Paragraph(
            "A fail-closed scientific-object, applicability, evidence, and process contract "
            "for Prisoma and its ecosystem consumers.",
            styles["cover_deck"],
        ),
        callout,
        Spacer(1, 12 * mm),
        status_table,
        Spacer(1, 13 * mm),
        Paragraph("METHOD AND PUBLICATION STAGES", styles["small"]),
        Spacer(1, 3 * mm),
        process,
        Spacer(1, 11 * mm),
        Paragraph(
            "Preserve valid novel work. Block inapplicable claims. Never use one PID as an "
            "automatic fallback for another.",
            styles["cover_deck"],
        ),
        PageBreak(),
    ]


def toc_story(styles: dict[str, ParagraphStyle]) -> list[object]:
    toc = TableOfContents()
    toc.levelStyles = [
        ParagraphStyle(
            "TOC0",
            fontName="PrisomaSans-Bold",
            fontSize=8.7,
            leading=11,
            leftIndent=0,
            firstLineIndent=0,
            textColor=NAVY,
            spaceBefore=2,
        ),
        ParagraphStyle(
            "TOC1",
            fontName="PrisomaSans",
            fontSize=8,
            leading=10,
            leftIndent=12,
            firstLineIndent=0,
            textColor=TEXT,
        ),
        ParagraphStyle(
            "TOC2",
            fontName="PrisomaSans",
            fontSize=7.5,
            leading=9.3,
            leftIndent=24,
            firstLineIndent=0,
            textColor=MID_GRAY,
        ),
        ParagraphStyle(
            "TOC3",
            fontName="PrisomaSans-Italic",
            fontSize=7,
            leading=8.8,
            leftIndent=36,
            firstLineIndent=0,
            textColor=MID_GRAY,
        ),
    ]
    return [
        Paragraph("Contents", styles["toc_title"]),
        Paragraph(
            "The table of contents is generated from the canonical Markdown headings.",
            styles["small"],
        ),
        Spacer(1, 4 * mm),
        toc,
        PageBreak(),
    ]


def render_equation(
    equation: str,
    *,
    index: int,
    work_dir: Path,
    available_width: float,
) -> KeepTogether:
    compact = re.sub(r"\s+", " ", equation).strip()
    compact = compact.replace(r"\mathbb E", r"\mathbb{E}")
    compact = re.sub(r"\\mathcal\s+([A-Za-z])", r"\\mathcal{\1}", compact)
    compact = re.sub(r"\\mathbb\s+([A-Za-z])", r"\\mathbb{\1}", compact)
    for delimiter in (r"\bigl", r"\bigr", r"\Bigl", r"\Bigr"):
        compact = compact.replace(delimiter, "")
    target = work_dir / f"equation-{index:02d}.png"
    try:
        math_to_image(
            f"${compact}$",
            str(target),
            prop=FontProperties(family="STIXGeneral", size=13),
            dpi=220,
            format="png",
            color="#102A43",
        )
    except Exception as error:
        raise RenderError(
            f"cannot render display equation {index}: {compact}"
        ) from error
    image = Image(str(target))
    max_width = available_width - 18
    max_height = 27 * mm
    scale = min(max_width / image.imageWidth, max_height / image.imageHeight, 1.0)
    image.drawWidth = image.imageWidth * scale
    image.drawHeight = image.imageHeight * scale
    caption = Paragraph(
        "<b>Canonical source expression:</b> "
        f'<font name="PrisomaMono">{html.escape(compact, quote=True)}</font>',
        ParagraphStyle(
            f"EquationCaption{index}",
            fontName="PrisomaSans",
            fontSize=5.8,
            leading=7.4,
            textColor=MID_GRAY,
            splitLongWords=True,
        ),
    )
    table = Table([[image], [caption]], colWidths=[available_width])
    table.setStyle(
        TableStyle(
            [
                ("BACKGROUND", (0, 0), (-1, -1), PALE_BLUE),
                ("BOX", (0, 0), (-1, -1), 0.5, LIGHT_GRAY),
                ("ALIGN", (0, 0), (-1, -1), "CENTER"),
                ("ALIGN", (0, 1), (0, 1), "LEFT"),
                ("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
                ("TOPPADDING", (0, 0), (-1, -1), 7),
                ("BOTTOMPADDING", (0, 0), (-1, 0), 3),
                ("LEFTPADDING", (0, 1), (0, 1), 9),
                ("RIGHTPADDING", (0, 1), (0, 1), 9),
                ("TOPPADDING", (0, 1), (0, 1), 2),
                ("BOTTOMPADDING", (0, 1), (0, 1), 6),
            ]
        )
    )
    return KeepTogether([table])


def table_widths(columns: int, available_width: float) -> list[float]:
    fractions_by_columns = {
        2: (0.29, 0.71),
        3: (0.21, 0.48, 0.31),
        4: (0.19, 0.39, 0.20, 0.22),
    }
    fractions = fractions_by_columns.get(columns)
    if fractions is None:
        fractions = tuple(1 / columns for _ in range(columns))
    return [available_width * fraction for fraction in fractions]


def make_table(
    rows: list[list[str]],
    *,
    styles: dict[str, ParagraphStyle],
    available_width: float,
) -> Table:
    if not rows or len(rows) > MAX_TABLE_ROWS:
        raise RenderError("table row count is outside the renderer bound")
    columns = len(rows[0])
    if columns < 2 or columns > MAX_TABLE_COLUMNS:
        raise RenderError("table column count is outside the renderer bound")
    if any(len(row) != columns for row in rows):
        raise RenderError("table rows have inconsistent column counts")
    rendered: list[list[Paragraph]] = []
    for row_index, row in enumerate(rows):
        style = styles["table_header"] if row_index == 0 else styles["table"]
        markup_options = (
            {"code_color": "#FFFFFF", "math_color": "#FFFFFF", "link_color": "#FFFFFF"}
            if row_index == 0
            else {}
        )
        rendered.append(
            [Paragraph(inline_markup(cell, **markup_options), style) for cell in row]
        )
    table = Table(
        rendered,
        colWidths=table_widths(columns, available_width),
        repeatRows=1,
        splitByRow=1,
        hAlign="LEFT",
    )
    commands: list[tuple[object, ...]] = [
        ("BACKGROUND", (0, 0), (-1, 0), NAVY),
        ("BOX", (0, 0), (-1, -1), 0.5, LIGHT_GRAY),
        ("INNERGRID", (0, 0), (-1, -1), 0.3, LIGHT_GRAY),
        ("VALIGN", (0, 0), (-1, -1), "TOP"),
        ("LEFTPADDING", (0, 0), (-1, -1), 4),
        ("RIGHTPADDING", (0, 0), (-1, -1), 4),
        ("TOPPADDING", (0, 0), (-1, -1), 4),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 4),
    ]
    for row_index in range(1, len(rows)):
        commands.append(
            (
                "BACKGROUND",
                (0, row_index),
                (-1, row_index),
                WHITE if row_index % 2 else PALE_GRAY,
            )
        )
    table.setStyle(TableStyle(commands))
    return table


def collect_list(lines: list[str], start: int) -> tuple[list[str], int, bool]:
    first = LIST_RE.match(lines[start])
    if first is None:
        raise RenderError("internal list parser error")
    ordered = first.group("marker").endswith(".") and first.group("marker")[0].isdigit()
    items: list[str] = []
    current = first.group("body").strip()
    index = start + 1
    while index < len(lines):
        line = lines[index]
        match = LIST_RE.match(line)
        if match is not None:
            next_ordered = (
                match.group("marker").endswith(".")
                and match.group("marker")[0].isdigit()
            )
            if next_ordered != ordered:
                break
            items.append(current)
            current = match.group("body").strip()
            index += 1
            continue
        if not line.strip():
            break
        if STRUCTURAL_RE.match(line):
            break
        current = f"{current} {line.strip()}"
        index += 1
    items.append(current)
    return items, index, ordered


def make_reference_grid(
    items: list[str],
    *,
    style: ParagraphStyle,
    available_width: float,
) -> Table:
    """Lay out a compact bibliography without creating an isolated spill page."""

    rows: list[list[Paragraph]] = []
    for item_index in range(0, len(items), 2):
        cells: list[Paragraph] = []
        for column in range(2):
            index = item_index + column
            if index < len(items):
                cells.append(
                    Paragraph(
                        '<font color="#1769AA">•</font> ' + inline_markup(items[index]),
                        style,
                    )
                )
            else:
                cells.append(Paragraph("", style))
        rows.append(cells)
    table = Table(
        rows,
        colWidths=[available_width / 2, available_width / 2],
        hAlign="LEFT",
        splitByRow=1,
    )
    table.setStyle(
        TableStyle(
            [
                ("VALIGN", (0, 0), (-1, -1), "TOP"),
                ("LEFTPADDING", (0, 0), (-1, -1), 0),
                ("RIGHTPADDING", (0, 0), (-1, -1), 7),
                ("TOPPADDING", (0, 0), (-1, -1), 0),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 2),
            ]
        )
    )
    return table


def markdown_story(
    source: str,
    *,
    styles: dict[str, ParagraphStyle],
    work_dir: Path,
) -> tuple[list[object], int, int, int]:
    lines = source.splitlines()
    story: list[object] = []
    available_width = PAGE_WIDTH - LEFT_MARGIN - RIGHT_MARGIN
    equation_count = 0
    table_count = 0
    heading_count = 0
    index = 0
    skipped_document_title = False
    primary_sources = False
    while index < len(lines):
        line = lines[index]
        stripped = line.strip()
        if not stripped:
            index += 1
            continue
        heading = re.match(r"^(#{1,4})\s+(.+)$", line)
        if heading is not None:
            level = len(heading.group(1))
            title = heading.group(2).strip()
            if level == 2:
                primary_sources = title == "15. Primary sources"
            if level == 1 and not skipped_document_title:
                skipped_document_title = True
                index += 1
                continue
            paragraph = Paragraph(inline_markup(title), styles[f"h{level}"])
            paragraph.bookmark_name = f"section-{heading_count:03d}"
            story.append(paragraph)
            heading_count += 1
            index += 1
            continue
        if stripped == r"\[":
            equation_lines: list[str] = []
            index += 1
            while index < len(lines) and lines[index].strip() != r"\]":
                equation_lines.append(lines[index].strip())
                index += 1
            if index >= len(lines):
                raise RenderError("unterminated display equation")
            equation_count += 1
            story.append(
                render_equation(
                    " ".join(equation_lines),
                    index=equation_count,
                    work_dir=work_dir,
                    available_width=available_width,
                )
            )
            story.append(Spacer(1, 3 * mm))
            index += 1
            continue
        if stripped.startswith("```"):
            code_lines: list[str] = []
            index += 1
            while index < len(lines) and not lines[index].strip().startswith("```"):
                code_lines.append(lines[index])
                index += 1
            if index >= len(lines):
                raise RenderError("unterminated fenced code block")
            code = html.escape("\n".join(code_lines), quote=False)
            code_box = Table(
                [[Preformatted(code, styles["code"])]],
                colWidths=[available_width],
            )
            code_box.setStyle(
                TableStyle(
                    [
                        ("BACKGROUND", (0, 0), (-1, -1), PALE_GRAY),
                        ("BOX", (0, 0), (-1, -1), 0.5, LIGHT_GRAY),
                        ("LEFTPADDING", (0, 0), (-1, -1), 6),
                        ("RIGHTPADDING", (0, 0), (-1, -1), 6),
                        ("TOPPADDING", (0, 0), (-1, -1), 5),
                        ("BOTTOMPADDING", (0, 0), (-1, -1), 5),
                    ]
                )
            )
            story.extend([code_box, Spacer(1, 3 * mm)])
            index += 1
            continue
        if stripped.startswith("|") and index + 1 < len(lines):
            if TABLE_SEPARATOR_RE.match(lines[index + 1]):
                rows = [split_table_row(line)]
                index += 2
                while index < len(lines) and lines[index].strip().startswith("|"):
                    rows.append(split_table_row(lines[index]))
                    index += 1
                table_count += 1
                story.extend(
                    [
                        make_table(
                            rows, styles=styles, available_width=available_width
                        ),
                        Spacer(1, 4 * mm),
                    ]
                )
                continue
        list_match = LIST_RE.match(line)
        if list_match is not None:
            items, index, ordered = collect_list(lines, index)
            if primary_sources and not ordered:
                story.append(
                    make_reference_grid(
                        items,
                        style=styles["reference"],
                        available_width=available_width,
                    )
                )
                continue
            list_style = (
                styles["reference"]
                if primary_sources and not ordered
                else styles["bullet"]
            )
            flowables = [
                ListItem(Paragraph(inline_markup(item), list_style), leftIndent=10)
                for item in items
            ]
            story.append(
                ListFlowable(
                    flowables,
                    bulletType="1" if ordered else "bullet",
                    start="1" if ordered else "•",
                    leftIndent=17,
                    bulletFontName="PrisomaSans-Bold",
                    bulletFontSize=7 if primary_sources and not ordered else 7.5,
                    bulletColor=BLUE,
                    spaceAfter=3 if primary_sources and not ordered else 6,
                )
            )
            continue
        if stripped.startswith(">"):
            quote_lines: list[str] = []
            while index < len(lines) and lines[index].strip().startswith(">"):
                quote_lines.append(lines[index].strip()[1:].strip())
                index += 1
            story.append(
                Paragraph(inline_markup(" ".join(quote_lines)), styles["quote"])
            )
            continue
        paragraph_lines = [stripped]
        index += 1
        while index < len(lines):
            candidate = lines[index]
            if not candidate.strip() or STRUCTURAL_RE.match(candidate):
                break
            paragraph_lines.append(candidate.strip())
            index += 1
        story.append(
            Paragraph(inline_markup(" ".join(paragraph_lines)), styles["body"])
        )
    return story, equation_count, table_count, heading_count


def render(source_path: Path, output_path: Path, work_dir: Path) -> RenderSummary:
    source_bytes, source = read_bounded_utf8(source_path, MAX_SOURCE_BYTES)
    renderer_bytes = Path(__file__).read_bytes()
    if work_dir.exists():
        shutil.rmtree(work_dir)
    work_dir.mkdir(parents=True, mode=0o755, exist_ok=True)
    output_path.parent.mkdir(parents=True, mode=0o755, exist_ok=True)
    register_fonts()
    styles = build_styles()
    content, equations, tables, headings = markdown_story(
        source,
        styles=styles,
        work_dir=work_dir,
    )
    story = cover_story(styles) + toc_story(styles) + content
    document = ContractDocTemplate(str(output_path), styles=styles)
    document.multiBuild(story)
    pdf_bytes = output_path.read_bytes()
    return RenderSummary(
        source_sha256=sha256_bytes(source_bytes),
        source_bytes=len(source_bytes),
        renderer_sha256=sha256_bytes(renderer_bytes),
        renderer_bytes=len(renderer_bytes),
        pdf_sha256=sha256_bytes(pdf_bytes),
        pdf_bytes=len(pdf_bytes),
        equation_count=equations,
        table_count=tables,
        heading_count=headings,
    )


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--work-dir", type=Path, default=DEFAULT_WORK_DIR)
    parser.add_argument(
        "--check",
        action="store_true",
        help="render to a temporary PDF and require byte identity with --output",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    source = args.source.resolve()
    output = args.output.resolve()
    work_dir = args.work_dir.resolve()
    if not source.is_file():
        raise RenderError(f"source does not exist: {source}")
    if args.check:
        if not output.is_file():
            raise RenderError(f"expected PDF does not exist: {output}")
        check_output = (
            work_dir / "check/PID_Method_Selection_and_Publication_Contract.pdf"
        )
        summary = render(source, check_output, work_dir / "check-equations")
        expected = output.read_bytes()
        observed = check_output.read_bytes()
        if observed != expected:
            raise RenderError(
                "rendered PDF differs from the checked publication view: "
                f"expected {sha256_bytes(expected)}, observed {sha256_bytes(observed)}"
            )
    else:
        summary = render(source, output, work_dir / "equations")
    print(json.dumps(summary.__dict__, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    rl_config.invariant = 1
    try:
        raise SystemExit(main(sys.argv[1:]))
    except (OSError, RenderError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        raise SystemExit(1) from None
