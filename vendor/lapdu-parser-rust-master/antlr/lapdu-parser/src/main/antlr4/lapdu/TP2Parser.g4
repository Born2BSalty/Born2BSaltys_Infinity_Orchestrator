parser grammar TP2Parser;

options {
	tokenVocab = TP2Lexer;
}

import TP2Rules;

rootRule
:
	tp2FileRule
;