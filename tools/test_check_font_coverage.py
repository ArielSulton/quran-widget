"""Tests for tools/check_font_coverage.py: the bundled Amiri Quran must cover every code point we render."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_font_coverage as cfc  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
FONT = ROOT / "src" / "assets" / "fonts" / "AmiriQuran-Regular.woff2"


def _tiny_font(tmp_path, chars, with_mark=True):
    """A minimal TTF with the given characters (and optionally mark/mkmk GPOS features)."""
    from fontTools.fontBuilder import FontBuilder
    from fontTools.pens.ttGlyphPen import TTGlyphPen
    from fontTools.feaLib.builder import addOpenTypeFeaturesFromString

    names = [".notdef"] + [f"uni{ord(c):04X}" for c in chars]
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(names)
    fb.setupCharacterMap({ord(c): f"uni{ord(c):04X}" for c in chars})
    empty = TTGlyphPen(None).glyph()
    fb.setupGlyf({n: empty for n in names})
    fb.setupHorizontalMetrics({n: (500, 0) for n in names})
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    fb.setupNameTable({"familyName": "Tiny", "styleName": "Regular"})
    fb.setupOS2()
    fb.setupPost()
    if with_mark and len(chars) >= 2:
        base, mark = f"uni{ord(chars[0]):04X}", f"uni{ord(chars[1]):04X}"
        fea = f"""
        markClass [{mark}] <anchor 0 600> @TOP;
        feature mark {{ pos base [{base}] <anchor 0 800> mark @TOP; }} mark;
        feature mkmk {{ pos mark [{mark}] <anchor 0 100> mark @TOP; }} mkmk;
        """
        addOpenTypeFeaturesFromString(fb.font, fea)
    path = tmp_path / "tiny.ttf"
    fb.save(str(path))
    return path


def test_used_codepoints_come_from_data_and_basmalah():
    cps = cfc.used_codepoints()
    assert 0x0627 in cps          # alef
    assert 0x0671 in cps          # alef wasla (tofu in fallback fonts)
    assert 0x06E5 in cps          # small waw
    assert 0x0020 in cps
    assert len(cps) > 40


def test_bundled_font_covers_all_used_codepoints_and_has_mark_positioning():
    assert cfc.check_font(FONT, cfc.used_codepoints()) == []


def test_bundled_font_also_covers_the_whole_quranic_annotation_range():
    """Juz 30 uses U+06D6, U+06DA, U+06DC and U+06E9; the rest of the range is covered for later juz."""
    quranic = set(range(0x06D6, 0x06DD)) | set(range(0x06DF, 0x06E9)) | set(range(0x06EA, 0x06EE))
    quranic.add(0x0670)
    assert cfc.check_font(FONT, quranic) == []


def test_bundled_font_identity_and_license():
    info = cfc.font_identity(FONT)
    assert info["family"] == "Amiri Quran"
    assert "SIL Open Font License" in info["license"]
    assert "Amiri Project Authors" in info["copyright"]


def test_missing_codepoint_is_reported(tmp_path):
    font = _tiny_font(tmp_path, "ab")
    problems = cfc.check_font(font, {ord("a"), ord("z")}, require_identity=False)
    assert any("U+007A" in p for p in problems)
    assert not any("U+0061" in p for p in problems)


def test_missing_mark_positioning_is_reported(tmp_path):
    font = _tiny_font(tmp_path, "ab", with_mark=False)
    problems = cfc.check_font(font, {ord("a")}, require_identity=False)
    assert any("mark" in p for p in problems)
    assert any("mkmk" in p for p in problems)


def test_font_with_mark_features_has_no_mark_problem(tmp_path):
    font = _tiny_font(tmp_path, "ab", with_mark=True)
    assert cfc.check_font(font, {ord("a")}, require_identity=False) == []


def test_wrong_family_is_reported(tmp_path):
    font = _tiny_font(tmp_path, "ab")
    problems = cfc.check_font(font, {ord("a")})
    assert any("Amiri Quran" in p for p in problems)
