parser grammar LapduCombinedParser;

import TP2Rules, DRules, TraRules;

options {
	tokenVocab = LapduCombinedLexer;
}

dFile
:
	dFileRule
;

tp2File
:
	tp2FileRule
;

traFile
:
	traFileRule
;

