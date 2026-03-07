lexer grammar TP2Lexer;

import Fragments, TP2InlineFile, Strings, Comments, TP2Keywords, Numbers, SharedKeywords, Ids;

// this grammar is used purely for includes and it needs one rule to stop antl4 complains

WHITESPACE
:
	[ \r\n\t]+ -> skip
;
