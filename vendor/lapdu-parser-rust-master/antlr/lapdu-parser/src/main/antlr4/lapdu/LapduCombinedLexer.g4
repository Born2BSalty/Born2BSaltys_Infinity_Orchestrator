lexer grammar LapduCombinedLexer;

import
TP2Keywords, TP2InlineFile, DKeywords, Fragments, Strings, Comments, TP2Keywords, Numbers, SharedKeywords, Ids;

// this grammar is used purely for includes and it needs one rule to stop antl4 complains

WHITESPACE
:
	[ \r\n\t]+ -> skip
;
