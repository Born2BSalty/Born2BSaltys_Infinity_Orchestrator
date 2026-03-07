lexer grammar Ids;

import Fragments;

// this rule matches basically any keyword which means it should always be imported **LAST**

IDENTIFIER
:
	(
		ALPHANUM
		| '_'
	)
	(
		ALPHANUM
		| '-'
		| '_'
		| '#'
		| '.'
		| '\''
	)*
;