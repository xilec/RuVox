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

URL components (host labels, path segments, query keys and values, fragment)
and email local parts SHALL be percent-decoded after the structural splits
(`/`, `&`, `=`, `@`) and before lexical processing: valid percent-encoded
UTF-8 sequences SHALL decode to their text (so an encoded Cyrillic file name
is read as that name), `%20` and other encoded ASCII SHALL decode to their
characters read by the existing rules, and encoded characters MUST NOT
change the URL structure (a decoded `%2F` is not a path separator). A `+`
inside query components SHALL be read as a word separator (space); a `+` in
email local parts, path segments, and fragments SHALL be read as "плюс".
Invalid percent sequences (truncated, non-hex, or non-UTF-8 byte runs) MUST
NOT leak a literal `%`: the `%` SHALL be read as "процент" and the following
characters read normally. No literal `%` or `+` SHALL remain in the
normalized output.

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

#### Scenario: Percent-encoded space in path

- GIVEN the input "https://example.com/hello%20world"
- WHEN the pipeline processes it
- THEN the output contains "слэш хеллоу ворлд" ("hello" via `IT_TERMS`) and
  no literal "%" remains

#### Scenario: Percent-encoded Cyrillic file name

- GIVEN the input "https://example.com/%D1%84%D0%B0%D0%B9%D0%BB"
- WHEN the pipeline processes it
- THEN the output contains "слэш файл" (the bytes decode to "файл")

#### Scenario: Plus in query is a space

- GIVEN the input "https://example.com/search?q=hello+world"
- WHEN the pipeline processes it
- THEN the output contains "к равно хеллоу ворлд" and no literal "+" remains

#### Scenario: Plus in email local part

- GIVEN the input "user+tag@example.com"
- WHEN the pipeline processes it
- THEN the output contains "юзер плюс таг собака экзампл точка ком" and no
  literal "+" remains

#### Scenario: Encoded percent sign and truncated sequence

- GIVEN the input "https://example.com/100%25done%2"
- WHEN the pipeline processes it
- THEN "%25" decodes to "%" read as "процент", the truncated "%2" reads its
  "%" as "процент" followed by "два", and no literal "%" remains
