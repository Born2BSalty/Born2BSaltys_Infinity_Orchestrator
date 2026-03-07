lexer grammar DLexer;

import DKeywords, Fragments, Numbers, Strings, Comments, SharedKeywords, Ids;

// this grammar is used purely for includes and it needs one rule to stop antl4 complains

WHITESPACE
:
	[ \r\n\t]+ -> skip
;
