#!/usr/bin/env python3
"""XLSX to PDF converter using xlsx2html + weasyprint.

Replaces LibreOffice's `soffice --headless --convert-to pdf` for spreadsheets.

Pipeline: XLSX → xlsx2html → HTML (with CSS styling) → weasyprint → PDF

Usage:
    python xlsx_to_pdf.py <input.xlsx> <output.pdf>

Dependencies (pip):
    xlsx2html, weasyprint, openpyxl
"""

import sys
import os
import tempfile
from pathlib import Path


def xlsx_to_pdf(input_path: str, output_path: str) -> None:
    from xlsx2html import xlsx2html as convert_xlsx
    from weasyprint import HTML
    import openpyxl

    input_path = os.path.abspath(input_path)
    output_path = os.path.abspath(output_path)

    if not os.path.exists(input_path):
        print(f"Error: Input file not found: {input_path}", file=sys.stderr)
        sys.exit(1)

    # Load workbook to get sheet names
    wb = openpyxl.load_workbook(input_path, read_only=True, data_only=True)
    sheet_names = wb.sheetnames
    wb.close()

    html_parts = []

    # CSS for page layout and table styling
    css = """
    @page {
        size: landscape;
        margin: 0.5in;
    }
    body {
        font-family: Arial, Helvetica, sans-serif;
        font-size: 10pt;
        margin: 0;
        padding: 0;
    }
    .sheet-title {
        font-size: 14pt;
        font-weight: bold;
        margin: 0 0 8pt 0;
        padding: 4pt 0;
        border-bottom: 1px solid #333;
        page-break-before: always;
    }
    .sheet-title:first-child {
        page-break-before: avoid;
    }
    table {
        border-collapse: collapse;
        width: 100%;
        table-layout: auto;
    }
    td, th {
        border: 1px solid #ccc;
        padding: 2pt 4pt;
        vertical-align: top;
        word-wrap: break-word;
        max-width: 200pt;
    }
    """

    for idx, sheet_name in enumerate(sheet_names):
        # Convert each sheet to HTML
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".html", delete=False, encoding="utf-8"
        ) as tmp:
            tmp_path = tmp.name

        try:
            convert_xlsx(input_path, tmp_path, sheet=idx)

            with open(tmp_path, "r", encoding="utf-8") as f:
                sheet_html = f.read()

            # Extract just the table content from xlsx2html output
            # xlsx2html generates a full HTML doc; we want the <table> part
            import re
            table_match = re.search(
                r"<table[\s\S]*?</table>", sheet_html, re.IGNORECASE
            )
            if table_match:
                table_html = table_match.group(0)
            else:
                # Fallback: use the body content
                body_match = re.search(
                    r"<body[^>]*>([\s\S]*?)</body>", sheet_html, re.IGNORECASE
                )
                table_html = body_match.group(1) if body_match else sheet_html

            # Add sheet title for multi-sheet workbooks
            if len(sheet_names) > 1:
                html_parts.append(
                    f'<div class="sheet-title">{sheet_name}</div>'
                )
            html_parts.append(table_html)
        finally:
            if os.path.exists(tmp_path):
                os.unlink(tmp_path)

    # Combine all sheets into one HTML document
    full_html = f"""<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>{css}</style>
</head>
<body>
{''.join(html_parts)}
</body>
</html>"""

    # Render to PDF with weasyprint
    HTML(string=full_html).write_pdf(output_path)

    if not os.path.exists(output_path):
        print("Error: PDF generation failed", file=sys.stderr)
        sys.exit(1)

    size = os.path.getsize(output_path)
    print(f"OK:{size}")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <input.xlsx> <output.pdf>", file=sys.stderr)
        sys.exit(1)
    xlsx_to_pdf(sys.argv[1], sys.argv[2])
