parser grammar TraRules;

options {
	tokenVocab = TraLexer;
}

import SharedRules;

traFileRule
:
	lines += traFileLineRule*
;

traFileLineRule
:
	ref = referenceIdRule EQ val = dlgLineRule
	// XXX: this is actually quite messy, as dlgLineRule can also match @0 references

;

referenceIdRule
:
	TRANSLATION_REFERENCE
;
