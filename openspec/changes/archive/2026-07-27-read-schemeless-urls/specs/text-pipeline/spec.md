# Delta spec: text-pipeline

## MODIFIED Requirements

### Requirement: URLs, emails, IP addresses, and file paths

The system SHALL normalize URLs with schemes `http`, `https`, `ftp`, `ssh`,
and `git` before any number processing: the scheme SHALL be read via the
protocol table ("https" → "эйч ти ти пи эс"), followed by "двоеточие слэш
слэш"; domain parts SHALL be joined with "точка" with known TLDs read via
`TLD_MAP` ("com" → "ком"); a numeric port SHALL follow "двоеточие"; path
segments SHALL follow "слэш"; query parameters SHALL follow "вопросительный
знак" with "=" read as "равно"; fragments SHALL follow "решётка". Alphabetic
segments SHALL be transliterated (via `IT_TERMS` first, then digraph-based
transliteration). Email addresses SHALL be read with "собака" for `@`, with
dots, underscores, hyphens, and digits in the local part read as "точка",
"андерскор", "дефис", and number words. IPv4 addresses SHALL be read as four
number words joined by "точка". File paths SHALL be read with "слэш" (or
"бэкслэш" for Windows paths), "тильда" for `~`, "точка" / "точка точка" for
`.` / `..`, drive letters plus "двоеточие" for Windows drives, and "точка"
before file extensions.

The system SHALL also detect scheme-less URLs — `www.`-prefixed domains and
bare domains whose last label is a known TLD from `TLD_MAP` — with optional
path, query, and fragment, and SHALL read them like schemed URLs without the
scheme prefix: domain labels joined with "точка" (known TLDs via `TLD_MAP`),
path segments after "слэш", query after "вопросительный знак", fragment
after "решётка". Detection MUST NOT match dotted names whose last label is
not in `TLD_MAP` (filenames like `file.txt`, `test.spec.ts`, `config.yaml`),
MUST NOT match numeric dotted forms (versions `1.2.3`, dates), and MUST NOT
re-match domains already consumed as part of a schemed URL or an email
address.

#### Scenario: HTTPS URL

- GIVEN the input "https://github.com/user/repo"
- WHEN the pipeline processes it
- THEN the output contains "эйч ти ти пи эс двоеточие слэш слэш гитхаб точка
  ком слэш юзер слэш репо"

#### Scenario: www-prefixed URL

- GIVEN the input "Сайт www.example.com недоступен"
- WHEN the pipeline processes it
- THEN the output contains "ввв точка экзампл точка ком" and no literal "."
  between the domain labels remains

#### Scenario: Bare domain with path

- GIVEN the input "документация на docs.python.org/3/tutorial"
- WHEN the pipeline processes it
- THEN the output contains "докс точка пайтон точка орг слэш три слэш
  тьюториал"

#### Scenario: Filename is not a bare domain

- GIVEN the input "открой file.txt и config.yaml"
- WHEN the pipeline processes it
- THEN neither name is read with "точка" domain separators (their suffixes
  are not in `TLD_MAP`)

#### Scenario: Version is not a bare domain

- GIVEN the input "версия 1.2.3"
- WHEN the pipeline processes it
- THEN the version is read as "один точка два точка три" by the version
  phase, not treated as a domain

#### Scenario: Email address

- GIVEN the input "user@example.com"
- WHEN the pipeline processes it
- THEN the output contains "собака" between the local part and the domain,
  and the domain is read with "точка ком"

#### Scenario: Unix file path

- GIVEN the input "/home/user/file.txt"
- WHEN the pipeline processes it
- THEN the output reads the path with "слэш" separators and "точка" before
  the extension

#### Scenario: Windows path with drive letter

- GIVEN the input "C:\Users\Admin\file.txt"
- WHEN the pipeline processes it
- THEN the output starts with "си двоеточие бэкслэш"
