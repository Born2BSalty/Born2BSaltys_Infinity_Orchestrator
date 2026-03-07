parser grammar SharedRules;

fileRule
:
	stringRule
;

sayTextRule
:
	dlgLineRule
;

traLineRule
:
	string = stringRule
	| ref = referenceRule
	| dlgLine = dlgLineRule // XXX: weidu accepts gendered and voiced lines even if does not make sense to do so - like in mod name or error display messages

;

dlgLineRule
:
	(
		forcedIndex = FORCED_STRING_REFERENCE
	)? maleLine = stringRule maleSound = soundRule? femaleLine = stringRule
	(
		forcedIndex = FORCED_STRING_REFERENCE
	)? femaleSound = soundRule? # genderedText
	| line = stringRule sound = soundRule? # genderNeutralText
	| referenceRule # referencedText
;

stringRule
:
	parts += stringLiteralRule
	(
		CONCAT parts += stringLiteralRule
	)*
;

stringLiteralRule
:
	identifierRule # unquotedStringLiteral
	| PERCENT_STRING # percentStringLiteral
	| TILDE_STRING # tildeStringLiteral
	| LONG_TILDE_STRING # longTildeStringLiteral
	| QUOTE_STRING # quotedStringLiteral
;

identifierRule
:
	IDENTIFIER
;

referenceRule
:
	SHARP_NUMBER # sharpReference
	| TRANSLATION_REFERENCE # traReference
	| PAREN_OPEN AT stringRule PAREN_CLOSE # varTraReference
	// TODO: add support for invalid non-numeric references like @asd

;

sharpNumberRule
:
	SHARP_NUMBER
;

soundRule
:
	SOUND_STRING
;