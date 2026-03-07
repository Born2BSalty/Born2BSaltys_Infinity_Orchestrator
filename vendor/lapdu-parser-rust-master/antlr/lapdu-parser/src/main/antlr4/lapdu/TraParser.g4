parser grammar TraParser;

options {
	tokenVocab = TraLexer;
}

import TraRules;

rootRule
:
	traFileRule
;