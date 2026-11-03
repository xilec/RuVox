## MODIFIED Requirements

### Requirement: Licensing and attribution

The bundle SHALL ship `LICENSE`, `LICENSE-OPENRAIL` and
`RUACCENT_NOTICE.txt`. The repository SHALL ship `teratts/NOTICE` identifying
the upstream model, the fine-tuning provenance, the modifications made for the
bundle, and the use-based restrictions. `README.md` and `README.en.md` SHALL
carry the commercial attribution "Text-to-speech powered by TeraTTSv2 by
TeraSpace" with a link, and an accurate license section: Silero Native stays
non-commercial, TeraTTSv2 — the default engine — is the commercially usable
option under OpenRAIL-M with its attribution condition, and RUAccent stays MIT
with its notice.

#### Scenario: License files are present in the bundle

- GIVEN a downloaded bundle
- WHEN its root is listed
- THEN `LICENSE`, `LICENSE-OPENRAIL` and `RUACCENT_NOTICE.txt` exist

#### Scenario: Attribution is present in both READMEs

- GIVEN `README.md` and `README.en.md`
- WHEN the attribution string is searched
- THEN both contain it with a link to the TeraSpace model
