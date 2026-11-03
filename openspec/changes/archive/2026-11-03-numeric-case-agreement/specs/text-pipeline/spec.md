## ADDED Requirements

### Requirement: Numeric noun case agreement

The numeric span layer SHALL decline the noun that follows a number in the case
the preposition selects: after "к"/"ко" the dative, after "о"/"об"/"при" the
prepositional, for units, currencies, percentages, scale words and fraction
tails, declining both the numeral and the noun ("к 5 кг" → "к пяти килограммам",
"о 2 кг" → "о двух килограммах", "при 10 %" → "при десяти процентах", "к $5" →
"к пяти долларам", "к 1 млн" → "к миллиону", "к 2 тыс." → "к двум тысячам"). An
indeclinable noun SHALL keep its form while the numeral declines ("к €5" → "к
пяти евро"), and a fixed unit SHALL stay fixed with the numeral declined ("к 1
°C" → "к одному градусу Цельсия").

An adjectival fraction SHALL keep a following unit in the singular genitive
("около 1/2 кг" → "около одной второй килограмма"), while a simple fraction
SHALL decline the unit with the surrounding case ("к 1,5 кг" → "к полутора
килограммам").

#### Scenario: Dative after к and ко

- GIVEN the inputs "к 5 кг", "к $5" and "к 1 млн"
- WHEN the pipeline processes them with the reference profile
- THEN both the numeral and the noun take the dative: "к пяти килограммам", "к
  пяти долларам" and "к миллиону"

#### Scenario: Prepositional after о, об and при

- GIVEN the inputs "о 2 кг", "об 1 кг" and "при 10 %"
- WHEN the pipeline processes them with the reference profile
- THEN both the numeral and the noun take the prepositional: "о двух
  килограммах", "об одном килограмме" and "при десяти процентах"

#### Scenario: Indeclinable and fixed nouns

- GIVEN the inputs "к €5" and "к 1 °C"
- WHEN the pipeline processes them with the reference profile
- THEN the numeral declines and the noun stays fixed: "к пяти евро" and "к
  одному градусу Цельсия"

#### Scenario: Fraction and unit case

- GIVEN the inputs "около 1/2 кг" and "к 1,5 кг"
- WHEN the pipeline processes them with the reference profile
- THEN an adjectival fraction keeps the singular genitive ("около одной второй
  килограмма") and a simple fraction declines the unit ("к полутора
  килограммам")
