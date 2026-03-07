lexer grammar Comments;

tokens {
	BLOCK_COMMENT
}

LINE_COMMENT
:
	'/' '/' ~[\n]* -> skip
;

BLOCK_COMMENT_START
:
	'/*' -> pushMode ( BLOCK_COMMENT_MODE ) , more
;

mode BLOCK_COMMENT_MODE;

// XXX: Weidu allows nested block comments

BLOCK_COMMENT_NEST
:
	'/*' -> pushMode ( BLOCK_COMMENT_MODE ) , more
;

BLOCK_COMMENT_CHAR
:
	. -> more
;

BLOCK_COMMENT_END
:
	'*/' -> popMode , skip
;

UNTERMINATED_BLOCK_COMMENT
:
	EOF -> popMode
;

