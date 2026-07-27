# Delta: text-pipeline

## MODIFIED Requirements

### Requirement: Numbers

The system SHALL read standalone integers as Russian cardinal number words
("123" → "сто двадцать три"), including thousands, millions, and billions
with correct declension and gender agreement ("тысяча" feminine). The number
phase SHALL skip digits that are adjacent to another digit or a
Latin/Cyrillic letter, or that sit next to a dot acting as a digit
separator (a decimal/version separator: a dot directly adjacent to digits
on the side facing the number), because those regions are owned by the
earlier URL, size, date, version, range, and code-identifier phases. A
number immediately before a terminal dot (a period followed by whitespace,
end of text, or a non-digit character) SHALL be read normally — sentence
punctuation is not a separator. Non-integer input and integers that fail to
parse SHALL be left unchanged. Number replacements SHALL be applied
positionally by byte range, so a match that is a substring of another match
(e.g. "1" inside "10") MUST NOT corrupt the longer number.

#### Scenario: Plain number

- GIVEN the input "Версия 3"
- WHEN the pipeline processes it
- THEN the number is read as "три"

#### Scenario: Number adjacent to a letter is skipped

- GIVEN a token like "v1" left over from earlier phases
- WHEN the number phase runs
- THEN the digit next to the letter is not expanded by the number phase

#### Scenario: Ratio with a multi-digit number

- GIVEN the input "Счёт 10:1 в нашу пользу."
- WHEN the pipeline processes it
- THEN the output is "Счёт десять:один в нашу пользу." with both numbers
  read as whole number words

#### Scenario: Repeated numbers replaced independently

- GIVEN the input "33 3"
- WHEN the pipeline processes it
- THEN the output reads both numbers correctly as "тридцать три три"

#### Scenario: Number before a sentence-ending dot

- GIVEN the input "Встреча в 5."
- WHEN the pipeline processes it
- THEN the number is read as "пять" and the output is "Встреча в пять."

#### Scenario: Dot between digits remains a separator

- GIVEN a leftover fragment like "3.14" that earlier phases did not consume
- WHEN the number phase runs
- THEN neither "3" nor "14" is expanded by the number phase
