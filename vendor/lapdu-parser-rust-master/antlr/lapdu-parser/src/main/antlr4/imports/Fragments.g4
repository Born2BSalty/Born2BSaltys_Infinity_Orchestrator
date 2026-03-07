lexer grammar Fragments;

fragment
LETTER
:
	[a-zA-Z]
;

fragment
ALPHANUM
:
	LETTER
	| DEC_DIGIT
;

fragment
BIN_DIGIT
:
	[01]
;

fragment
OCT_DIGIT
:
	[0-7]
;

fragment
DEC_DIGIT
:
	[0-9]
;

fragment
HEX_DIGIT
:
	DEC_DIGIT
	| [a-fA-F]
;

fragment
OCT_LITERAL_PREFIX
:
	'0o'
;

fragment
BIN_LITERAL_PREFIX
:
	'0b'
;

fragment
HEX_LITERAL_PREFIX
:
	'0x'
;
