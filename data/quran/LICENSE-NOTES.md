# Quran data: sources and license notes

Checked 2026-11-06. Intended use: redistribution inside a free, open, non-commercial desktop widget.

## (a) Arabic text: Tanzil Quran Text (Uthmani, v1.1)
- Source: https://tanzil.net/download/ (file `quran-uthmani.txt`, "Text with aya numbers"); terms at https://tanzil.net/docs/text_license
- License: Creative Commons Attribution 3.0. Copyright (C) 2007-2026 Tanzil Project. Terms from the file header:
  - "Permission is granted to copy and distribute verbatim copies of this text, but CHANGING IT IS NOT ALLOWED."
  - "This Quran text can be used in any website or application, provided that its source (Tanzil Project) is clearly indicated, and a link is made to tanzil.net to enable users to keep track of changes."
  - "This copyright notice shall be included in all verbatim copies of the text, and shall be reproduced appropriately in all files derived from or containing substantial portion of this text."
- Conclusion: redistribution allowed with attribution, link to tanzil.net and this notice. The text must stay unchanged. The only transformation applied is splitting into JSON and removing the basmala Tanzil prepends to ayah 1 of each surah (it is not part of ayah 1); no letter or diacritic of the ayah text is altered. The app must show "Teks Qur'an: Tanzil Project (tanzil.net)" in an About/credits place.
- Text has full tashkeel (verified by `tools/check_quran_data.py`).

## (b) Indonesian translation: Kemenag RI, via Tanzil (`id.indonesian`)
- Source: https://tanzil.net/trans/id.indonesian (listed at https://tanzil.net/trans/). File metadata: "Name: Bahasa Indonesia; Translator: Indonesian Ministry of Religious Affairs; Last Update: June 4, 2010; Source: Tanzil.net". Changelog https://tanzil.net/trans/log/id.indonesian also attributes it to the Indonesian Ministry of Religious Affairs.
- Terms (https://tanzil.net/trans/): "The translations provided at this page are for non-commercial purposes only. If used otherwise, you need to obtain necessary permission from the translator or the publisher." Using more than three translations requires a link back to the translations page.
- Conclusion: permitted for NON-COMMERCIAL use, which fits a free widget. Not permitted if the app is sold, ad-supported or otherwise monetised without Kemenag's permission. Credit "Terjemahan: Kementerian Agama RI (via Tanzil.net)".
- Caveat: Tanzil states no separate license from Kemenag. Kemenag's own site (quran.kemenag.go.id) terms of use were searched but no explicit reuse/license statement was found. A Wikisource discussion shows disagreement on whether the Kemenag translation is a free government work (Indonesian copyright law). No official confirmation was found. Residual legal risk is low for a non-commercial app, but not zero.

## (c) Quran font
- Candidates with clear terms: Amiri / Amiri Quran (SIL Open Font License 1.1, "Copyright 2010-2022 The Amiri Project Authors", https://github.com/aliftype/amiri/blob/main/OFL.txt) permits bundling and redistribution with software, provided the OFL text and copyright are kept and the font is not sold on its own. Scheherazade New and Noto Naskh Arabic are also OFL 1.1.
- KFGQPC Uthmanic Hafs (King Fahd Complex) was not verified in this task; do not bundle it without checking its terms.
- Conclusion for Task 3+: use an OFL font (Amiri Quran recommended) and ship its OFL.txt. No font file is added in Task 2.

## Quran Foundation / quran.com / QUL
Not used: a usable source with clear terms (Tanzil) was found, and no additional source is needed.

## Reproducing the data
```
mkdir /tmp/quran-src   # new empty dir
curl -L -o /tmp/quran-src/quran-uthmani.txt "https://tanzil.net/pub/download/index.php?quranType=uthmani&outType=txt-2&agree=true"
curl -L -o /tmp/quran-src/id.indonesian.txt "https://tanzil.net/trans/?transID=id.indonesian&type=txt-2"
python3 -I tools/build_quran_data.py /tmp/quran-src/quran-uthmani.txt /tmp/quran-src/id.indonesian.txt data/quran/juz30.json
python3 -I tools/check_quran_data.py
```

## Tanzil copyright block (Arabic text file)
```
Tanzil Quran Text (Uthmani, Version 1.1)
Copyright (C) 2007-2026 Tanzil Project
License: Creative Commons Attribution 3.0
Terms of use: see quotes in section (a). Please check updates at: http://tanzil.net/updates/
```
