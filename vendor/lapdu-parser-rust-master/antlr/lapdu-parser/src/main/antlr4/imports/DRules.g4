parser grammar DRules;

options {
	tokenVocab = DLexer;
}

import SharedRules;

dFileRule
:
	actions += dActionRule* EOF
;

dActionRule
:
	BEGIN dlg = fileRule
	(
		nonPausing = stringRule
	)? states += stateRule* # beginDAction
	| APPEND ifExists = IF_FILE_EXISTS? dlg = fileRule states += stateRule* END #
	appendDAction
	| APPEND_EARLY ifExists = IF_FILE_EXISTS? dlg = fileRule states += stateRule*
	END # appendEarlyDAction
	| CHAIN
	(
		IF
		(
			WEIGHT weight = sharpNumberRule
		)? trigger = stringRule THEN
	)? ifExists = IF_FILE_EXISTS? dlg = fileRule label = stringRule body =
	chainActionBodyRule epilog = chainActionEpilogRule # chainDAction
	| INTERJECT dlg = fileRule label = stringRule globalVar = stringRule blocks += // XXX: weidu doc wants chainBody here instead of blocks* 
	chainActionBranchOrMonologRule* epilog = chainActionEpilogRule #
	interjectDAction
	|
	(
		v1 = INTERJECT_COPY_TRANS
		| v2 = INTERJECT_COPY_TRANS2
		| v3 = INTERJECT_COPY_TRANS3
		| v4 = INTERJECT_COPY_TRANS4
	) safe = SAFE? ifExists = IF_FILE_EXISTS? //XXX: IF_FILE_EXISTS token is not present in weidu doc but seems to be required
	dlg = fileRule label = stringRule globalVar = stringRule blocks +=
	chainActionBranchOrMonologRule* END // XXX: END token is not present in weidu doc but seem to be required 
 # interjectCopyTransDAction
	|
	(
		top = EXTEND_TOP
		| bottom = EXTEND_BOTTOM
	) dlg = fileRule states += stringRule* position = sharpNumberRule? transitions
	+= transitionRule* END # extendTopBottomDAction
	| ADD_STATE_TRIGGER dlg = fileRule labels += stringRule trigger = stringRule //FIXME: it is unclear whats the difference between stateids

	(
		labels += stringRule
	)* conditions += conditionRule* # addStateTriggerDAction
	| ADD_TRANS_TRIGGER dlg = fileRule labels += stringRule trigger = stringRule
	(
		labels += stringRule
	)*
	(
		DO tras += stringRule*
	)? conditions += conditionRule* # addTransTriggerDAction
	| ADD_TRANS_ACTION dlg = fileRule BEGIN labels += stringRule* END BEGIN tras
	+= stringRule* END action = stringRule conditions += conditionRule* #
	addTransActionDAction
	| REPLACE_TRANS_ACTION dlg = fileRule BEGIN labels += stringRule* END BEGIN
	tras += stringRule* END oldText = stringRule newText = stringRule conditions
	+= conditionRule* # replaceTransActionDAction
	| REPLACE_TRANS_TRIGGER dlg = fileRule BEGIN labels += stringRule* END BEGIN
	tras += stringRule* END oldText = stringRule newText = stringRule conditions
	+= conditionRule* # replaceTransTriggerDAction
	| ALTER_TRANS dlg = fileRule BEGIN labels += stringRule* END BEGIN tras +=
	stringRule* END BEGIN changes += alterTransCommand* END # alterTransDAction
	| REPLACE dlg = fileRule newStates += stateRule* END # replaceDAction
	| SET_WEIGHT dlg = fileRule label = stringRule weight = sharpNumberRule #
	setWeightDAction
	| REPLACE_SAY dlg = fileRule label = stringRule newVal = sayTextRule #
	replaceSayDAction
	| REPLACE_STATE_TRIGGER dlg = fileRule labels += stringRule trigger = //FIXME: it is unclear whats the difference between stateids
	stringRule labels += stringRule* conditions += conditionRule* #
	replaceStateTriggerDAction
	| REPLACE_TRIGGER_TEXT dlg = fileRule oldText = stringRule newText =
	stringRule conditions += conditionRule* # replaceTriggerTextDAction
	| REPLACE_TRIGGER_TEXT_REGEXP dlgRegexp = stringRule oldText = stringRule
	newText = stringRule conditions += conditionRule* #
	replaceTriggerTextRegexpDAction
	| REPLACE_ACTION_TEXT dlgs += fileRule oldText = stringRule newText =
	stringRule dlgs += fileRule* conditions += conditionRule* #
	replaceActionTextDAction
	| REPLACE_ACTION_TEXT_REGEXP dlgRegexps += stringRule oldText = stringRule
	newText = stringRule dlgRegexps += stringRule* conditions += conditionRule* #
	replaceActionTextRegexpDAction
	| REPLACE_ACTION_TEXT_PROCESS dlgs += fileRule oldText = stringRule newText =
	stringRule dlgs += fileRule* conditions += conditionRule* #
	replaceActionTextProcessDAction
	| REPLACE_ACTION_TEXT_PROCESS_REGEXP dlgRegexps += stringRule oldText =
	stringRule newText = stringRule dlgRegexps += stringRule* conditions +=
	conditionRule* # ReplaceActionTextProcessRegexpDAction
;

alterTransCommand
:
	type = stringRule newVal = stringRule
;

conditionRule
:
	(
		isIf = IF
		| isUnless = UNLESS
	) predicate = stringRule
;

stateRule
:
	IF
	(
		WEIGHT weight = sharpNumberRule
	)? trigger = stringRule
	(
		THEN
	)?
	(
		BEGIN
	)? label = stringRule SAY lines += sayTextRule
	(
		EQ lines += sayTextRule
	)* transitions += transitionRule* END # ifThenState
	| APPENDI dlg = fileRule states += stateRule* END # appendiState
	| CHAIN2 dlg = fileRule entryLabel = stringRule body = chain2BodyRule END // XXX: END token is not present in weidu doc but seem to be required 
	exitDlg = fileRule exitLabel = stringRule # chain2State
;

chain2BodyRule
:
	states += chainActionDlgStateRule
	(
		(
			EQ
			| EQEQ
		) states += chainActionDlgStateRule
	)*
;

transitionRule
:
	IF trigger = stringRule
	(
		THEN
	)? features += transitionFeatureRule* out = transitionTargetRule #
	ifThenTransition
	| PLUS trigger = stringRule? PLUS reply = sayTextRule features +=
	transitionFeatureRule* out = transitionTargetRule # replyTransition
	| COPY_TRANS safe = SAFE? dlg = fileRule label = stringRule #
	copyTransTransition
	| COPY_TRANS_LATE safe = SAFE? dlg = fileRule label = stringRule #
	copyTransLateTransition
;

transitionTargetRule // aka transNext in weidu

:
	(
		GOTO
		| PLUS
	) label = stringRule # gotoTransitionTarget
	| EXTERN ifExists = IF_FILE_EXISTS? dlg = fileRule label = stringRule #
	externTransitionTarget
	| EXIT # exitTransitionTarget
;

chainActionEpilogRule
:
	END dlg = fileRule label = stringRule # endChainActionEpilog
	| EXTERN dlg = fileRule label = stringRule # externChainActionEpilog
	| COPY_TRANS safe = SAFE? dlg = fileRule label = stringRule #
	copyTransChainActionEpilog
	| COPY_TRANS_LATE safe = SAFE? dlg = fileRule label = stringRule #
	copyTransLateChainActionEpilog
	| EXIT # exitChainActionEpilog
	| END transitions += transitionRule* # endWithTransitionsChainActionEpilog
;

transitionFeatureRule
:
	REPLY line = dlgLineRule # replyTransitionFeature
	| DO action = stringRule # doTransitionFeature
	| JOURNAL entry = dlgLineRule # journalTransitionFeature
	| SOLVED_JOURNAL entry = dlgLineRule # solvedJournalTransitionFeature
	| UNSOLVED_JOURNAL entry = dlgLineRule # unsolvedJournalTransitionFeature
	| FLAGS flags = stringRule # flagsTransitionFeature
;

// aka chainText aka c3_dialogue_unit

chainActionBodyRule
:
	initialSpeakerStates += chainActionDlgStateRule
	(
		EQ initialSpeakerStates += chainActionDlgStateRule
	)* blocks += chainActionBranchOrMonologRule*
;

chainActionBranchOrMonologRule
:
	BRANCH trigger = stringRule BEGIN
	(
		blocks += chainActionBranchOrMonologRule
	)* END # chainActionBranch
	| EQEQ ifExists = IF_FILE_EXISTS? dlg = fileRule states +=
	chainActionDlgStateRule
	(
		EQ states += chainActionDlgStateRule
	)* # chainActionMonolog
;

//XXX: this one is derived from weidu source, but apparetnly weidu is using the same 
// ast for chain and interject which makes syntax unreliable
// TODO: while this looks like a reasonable DRY approximation of grammar, the CST
// it produces is nested in an unhandy way (intiali speaker lines are distinct from other lines)
// and could be possibly improved

chainActionDlgStateRule
:
	(
		IF trigger = stringRule THEN? // XXX: weidu does not mention that this THEN is optional

	)? line = sayTextRule
	(
		DO action = stringRule
	)? // XXX: weidu documentation seem to be outdatedor incorrect and does not not mention this DO block, but chain3_list has it

;