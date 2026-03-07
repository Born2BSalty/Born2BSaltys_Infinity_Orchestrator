lexer grammar TraLexer;

import Fragments, Strings, Comments, Numbers, SharedKeywords, Ids;

// this grammar is used purely for includes and it needs one rule to stop antl4 complains

WHITESPACE
:
	[ \r\n\t]+ -> skip
;