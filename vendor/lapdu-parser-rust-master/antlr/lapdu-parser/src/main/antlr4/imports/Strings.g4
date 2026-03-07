lexer grammar Strings;

SOUND_STRING
:
	'['
	(
		.
	)*? ']'
;

TILDE_STRING
:
	'~'
	(
		.
	)*? '~'
;

QUOTE_STRING
:
	'"'
	(
		.
	)*? '"'
;

PERCENT_STRING
:
	'%'
	(
		.
	)*? '%'
;

FORCED_STRING_REFERENCE
:
	'!' [0-9]+
;

TRANSLATION_REFERENCE
:
	'@'
	(
		'-'
	)? [0-9]+
;

CONCAT
:
	'^'
;

LONG_TILDE_STRING_START
:
	'~~~~~' -> pushMode ( LONG_TILDE_STRING_MODE ) , more
;

mode LONG_TILDE_STRING_MODE;

LONG_TILDE_STRING_BODY
:
	. -> more
;

LONG_TILDE_STRING
:
	'~~~~~' -> popMode
;

LONG_TILDE_STRING_UNTERMINATED
:
	EOF -> popMode
;

