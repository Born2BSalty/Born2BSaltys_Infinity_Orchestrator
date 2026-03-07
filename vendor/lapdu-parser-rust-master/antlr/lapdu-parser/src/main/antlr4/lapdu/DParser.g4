parser grammar DParser;

import DRules;

options {
	tokenVocab = DLexer;
}

rootRule
:
	dFileRule
;

