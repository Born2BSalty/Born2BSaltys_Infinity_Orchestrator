parser grammar TP2Rules;

options {
	tokenVocab = TP2Lexer;
}

import SharedRules;

tp2FileRule
:
	backupRule? authorRule? flags += tp2FlagRule* languages += languageRule*
	components += componentRule* EOF
;

backupRule
:
	BACKUP path = stringRule
;

authorRule
:
	AUTHOR name = stringRule
;

tp2FlagRule
:
	AUTO_TRA path = stringRule # autoTra
	| ALLOW_MISSING files += fileRule* # allowMissing
	| ASK_EVERY_COMPONENT # askEveryComponent
	| ALWAYS actions += tp2ActionRule* END # always
	| README files += fileRule* # readme
	| UNINSTALL_ORDER order += stringRule* # uninstallOrder
	| MODDER debugInfoList += stringRule* # modder
	| VERSION value = traLineRule # version
	| SCRIPT_STYLE val = stringRule # scriptStyle
	| NO_IF_EVAL_BUG # noIfEvalBug
	| QUICK_MENU params = quickMenuParamsRule END # quickMenu
	| AUTO_EVAL_STRINGS # autoEvalStrings
;

quickMenuParamsRule
:
	directives += quickMenuDirectiveRule* entries += quickMenuEntryRule*
;

quickMenuDirectiveRule
:
	value = stringRule END
;

quickMenuEntryRule
:
	title = traLineRule BEGIN components += quickMenuComponentRule* END
;

quickMenuComponentRule
:
	component = numberRule
;

tpaOrTppFileRule
:
	tpaFileRule
	| tppFileRule
;

tpaFileRule
:
	tp2ActionRule* EOF
;

tppFileRule
:
	patchActionRule* EOF
;

tp2ActionRule
:
	COPY optNoBackupRule? optGlobRule? files += fromToFilePairRule+ patch +=
	patchActionRule* when += whenConditionRule* # copyAction
	| COPY_EXISTING optNoBackupRule? optGlobRule? files += fromToFilePairRule+
	patch += patchActionRule* when += whenConditionRule* # copyExistingAction
	| COPY_EXISTING_REGEXP optNoBackupRule? optGlobRule? files +=
	regexpFileTupleRule+ patch += patchActionRule* when += whenConditionRule* #
	copyExistingRegexpAction
	| COPY_LARGE optNoBackupRule? optGlobRule? files += fromToFilePairRule+ #
	copyLargeAction
	| COPY_RANDOM fileLists += fileListRule+ patch = patchActionRule* when =
	whenConditionRule* # copyRandomAction
	| COPY_ALL_GAM_FILES patch = patchActionRule* when = whenConditionRule* #
	copyAllGamFilesAction
	| MOVE optNoBackupRule? files += fromToFilePairRule+ # moveAction
	| MOVE optNoBackupRule? files += moveRegexFileExpressionRule+ #
	moveRegexpAction
	| DELETE optNoBackupRule? files += fileRule+ # deleteAction
	| DISABLE_FROM_KEY files += fileRule+ # disableFromKeyAction
	| CREATE type = stringRule
	(
		VERSION version = stringRule
	)? resource = stringRule patch += patchActionRule* # createAction
	| COMPILE evaluateBuffer = EVALUATE_BUFFER? files += fileRule+ patch +=
	patchActionRule*
	(
		USING traFiles += fileRule+
	)? # compileActionContext
	| CLEAR_MEMORY # clearMemoryAction
	| CLEAR_ARRAYS # clearArraysAction
	| CLEAR_CODES # clearCodesAction
	| CLEAR_INLINED # clearInlinedAction
	| CLEAR_EVERYTHING # clearEverythingAction
	| CLEAR_IDS_MAP # clearIdsMapAction
	| ACTION_CLEAR_ARRAY var = lValRule # actionClearArrayAction
	| SILENT # silentAction
	| VERBOSE # verboseAction
	| MKDIR dirs += fileRule+ # createDirectoryAction
	| RANDOM_SEED seed = numberRule # randomSeedAction
	| ACTION_READLN var = lValRule # actionReadlnAction
	| APPEND optNoBackupRule? fileName = fileRule value = stringRule when +=
	whenConditionRule* keepCrlf = KEEP_CRLF? # appendAction
	| APPEND_OUTER optNoBackupRule? fileName = fileRule value = stringRule when +=
	whenConditionRule* keepCrlf = KEEP_CRLF? # appendOuterAction
	| APPEND_COL optNoBackupRule? fileName = fileRule value = stringRule
	prependExpr = expressionRule? when += whenConditionRule* keepCrlf =
	KEEP_CRLF? # appendColAction
	| APPEND_COL_OUTER optNoBackupRule? fileName = fileRule value = stringRule
	prependExpr = expressionRule? when += whenConditionRule* keepCrlf =
	KEEP_CRLF? # appendColOuterAction
	| EXTEND_TOP existingFile = fileRule newFile = fileRule patch +=
	patchActionRule*
	(
		USING traFiles += fileRule+
	)? # extendTopAction
	| EXTEND_BOTTOM existingFile = fileRule newFile = fileRule patch +=
	patchActionRule*
	(
		USING traFiles += fileRule+
	)? # extendBottomAction
	| EXTEND_TOP_REGEXP existingFileRegexp = regexpRule newFile = fileRule patch
	+= patchActionRule*
	(
		USING traFiles += fileRule+
	)? # extendTopRegexpAction
	| EXTEND_BOTTOM_REGEXP existingFileRegexp = regexpRule newFile = fileRule
	patch += patchActionRule*
	(
		USING traFiles += fileRule+
	)? # extendBottomRegexpAction
	| ACTION_IF expr = expressionRule THEN? BEGIN actions += tp2ActionRule* END
	(
		(
			ELSE elseActions += tp2ActionRule
		)
		|
		(
			ELSE BEGIN elseActions += tp2ActionRule* END
		)
	)? # actionIfAction
	| ACTION_MATCH value = expressionRule WITH branches += matchBranchRule*
	DEFAULT defaultActions += tp2ActionRule* END # actionMatchAction
	| ACTION_TRY actions += tp2ActionRule+ WITH branches += matchBranchRule*
	DEFAULT defaultActions += tp2ActionRule* END # actionTryAction
	| ACTION_RERAISE # actionReraiseAction
	| AT_EXIT command = stringRule exact = EXACT? # atExitAction
	| AT_INTERACTIVE_EXIT command = stringRule exact = EXACT? #
	atInteractiveExitAction
	| AT_UNINSTALL command = stringRule exact = EXACT? # atUninstallAction
	| AT_INTERACTIVE_UNINSTALL command = stringRule exact = EXACT? #
	atInteractiveUninstallAction
	| AT_UNINSTALL_EXIT command = stringRule exact = EXACT? #
	atUninstallExitAction
	| AT_INTERACTIVE_UNINSTALL_EXIT command = stringRule exact = EXACT? #
	atInteractiveUninstallExitAction
	| AT_NOW expr = expressionRule? command = stringRule exact = EXACT? #
	atNowAction
	| AT_NOW_INTERACTIVE expr = expressionRule? command = stringRule exact =
	EXACT? # atNowInteractiveAction
	| MAKE_BIFF name = stringRule BEGIN regexpExprs += directoryFileRegexpRule+
	END # makeBiffAction
	| LOAD_TRA traFiles += fileRule* # loadTraAction
	| WITH_TRA traFiles += fileRule* BEGIN actions += tp2ActionRule+ END #
	withTraAction
	| WITH_SCOPE BEGIN actions += tp2ActionRule+ END # withScopeAction
	| UNINSTALL modName = stringRule modComponent = stringRule # uninstallAction
	| COPY_KIT oldKit = stringRule newKit = stringRule PAREN_OPEN changes +=
	copyKitTupleRule* PAREN_CLOSE # copyKitAction
	| ADD_KIT internalName = stringRule fiels += stringRule*
	/* FIXME: weidu refuses to parse actions that does not have exactly 13 fiels */
	SAY lowerName = dlgLineRule SAY mixedName = dlgLineRule SAY description =
	dlgLineRule # addKitAction
	| ADD_MUSIC internalMusicName = stringRule newMUSFile = fileRule #
	addMusicAction
	| ADD_SCHOOL name = stringRule removalString = traLineRule # addSchoolAction
	| ADD_SECTYPE name = stringRule removalString = traLineRule # addSecTypeAction
	| ADD_AREA_TYPE name = stringRule # addAreaTypeAction
	| ADD_PROJECTILE fileName = fileRule unknownPurposeString = stringRule? #
	addProjectile // FIXME: unknownPurposeString

	| ADD_SPELL splFileName = fileRule type = expressionRule level =
	expressionRule idsName = expressionRule patch += patchActionRule*
	(
		IF_EXISTING patchIfReplacingSpell += patchActionRule* END
	)?
	(
		ON_DISABLE patchDisabledSpell += patchActionRule* END
	)? # addSpellAction
	| ADD_JOURNAL existing = EXISTING? managed = MANAGED?
	(
		TITLE PAREN_OPEN title = dlgLineRule PAREN_CLOSE
	)? references += referenceRule*
	(
		USING traFiles += fileRule*
	)? # addJournalAction
	| STRING_SET strings += stringSetTupleRule*
	(
		USING traFileName = fileRule
	)? # stringSetAction
	| STRING_SET_EVALUATE strings += stringSetTupleRule*
	(
		USING traFileName = fileRule
	)? # stringSetEvaluateAction
	| STRING_SET_RANGE
	(
		(
			PAREN_OPEN minExpr = expressionRule PAREN_CLOSE
		)
		|
		(
			minRef = sharpNumberRule
		)
	)
	(
		(
			PAREN_OPEN maxExpr = expressionRule PAREN_CLOSE
		)
		|
		(
			maxRef = sharpNumberRule
		)
	) USING traFileName = fileRule # stringSetRangeAction
	| ALTER_TLK_RANGE from = expressionRule to = expressionRule BEGIN patch +=
	patchActionRule* END # alterTlkRangeAction
	| ALTER_TLK_LIST BEGIN values += expressionRule* END BEGIN patch +=
	patchActionRule* END # alterTlkListAction
	| ALTER_TLK BEGIN patch += patchActionRule* END # alterTlkAction
	| REQUIRE_FILE fileName = fileRule warning = traLineRule # requireFileAction
	| FORBID_FILE fileName = fileRule warning = traLineRule # forbidFileAction
	| FAIL displayMessage = traLineRule # failAction
	| ABORT displayMessage = traLineRule # abortAction
	| WARN displayMessage = traLineRule # warnAction
	| PRINT val = exprOrRefRule # printAction
	| LOG val = expressionRule # logAction
	| OUTER_TEXT_SPRINT var = lValRule val = expressionRule #
	outerTextSprintAction
	| OUTER_SPRINT var = lValRule val = exprOrRefRule # outerSprintAction
	| OUTER_SNPRINT idx = expressionRule var = lValRule val = expressionRule #
	outerSnprintAction
	| OUTER_SET assignment = varAssignmentRule # outerSetAction
	| OUTER_INNER_PATCH buffer = stringRule BEGIN patch += patchActionRule* END #
	outerInnerPatchAction
	| OUTER_PATCH buffer = stringRule BEGIN patch += patchActionRule* END #
	outerPatchAction
	| OUTER_INNER_PATCH_SAVE var = lValRule buffer = stringRule BEGIN patch +=
	patchActionRule* END # outerInnerPatchSaveAction
	| OUTER_PATCH_SAVE var = lValRule buffer = stringRule BEGIN patch +=
	patchActionRule* END # outerPatchSaveAction
	| OUTER_WHILE expr = expressionRule BEGIN actions += tp2ActionRule* END #
	outerWhileAction
	| OUTER_FOR PAREN_OPEN initPatch += patchActionRule* SEMICOLON expr =
	expressionRule SEMICOLON incrementPatch += patchActionRule* PAREN_CLOSE BEGIN
	actions += tp2ActionRule* END # outerForAction
	| ACTION_BASH_FOR regexpExpr = directoryFileRegexpRule BEGIN actions +=
	tp2ActionRule* END # actionBashFor
	| ACTION_DEFINE_ARRAY var = lValRule BEGIN vals += stringRule* END #
	actionDefineArrayAction
	| ACTION_SORT_ARRAY_INDICES var = lValRule
	(
		lexicographically = LEXICOGRAPHICALLY
		| numerically = NUMERICALLY
	) # actionSortArrayIndicesAction
	| GET_FILE_ARRAY var = lValRule directory = fileRule matchFilesRegexp =
	regexpRule # getFileArrayAction
	| GET_DIRECTORY_ARRAY var = lValRule directory = fileRule matchFilesRegexp =
	regexpRule # getDirectoryArrayAction
	| ACTION_DEFINE_ASSOCIATIVE_ARRAY var = lValRule BEGIN vals += arrayTupleRule*
	END # actionDefineAssociativeArrayAction
	| ACTION_FOR_EACH itemVar = lValRule IN items += expressionRule* BEGIN actions
	+= tp2ActionRule* END # actionForeachAction
	| ACTION_PHP_EACH val = expressionRule AS keyTemplate = lValRule FAT_ARROW
	valTemplate = lValRule
	/* actually weidu accepts a list of pairs, but the purpose is unclear */ BEGIN
	actions += tp2ActionRule* END # actionPhpEachAction
	|
	(
		INCLUDE
		| ACTION_INCLUDE
	) files += fileRule+ # includeAction
	|
	(
		REINCLUDE
		| ACTION_REINCLUDE
	) files += fileRule+ # reincludeAction
	| rawContent = INLINE_FILE # inlineFileAction
	| ACTION_TO_UPPER var = lValRule # actionToUpperAction
	| ACTION_TO_LOWER var = lValRule # actionToLowerAction
	| ACTION_GET_STRREF val = expressionRule var = lValRule #
	actionGetStringrefAction
	| ACTION_GET_STRREF_F val = expressionRule var = lValRule #
	actionGetStringrefFemaleAction
	| ACTION_GET_STRREF_S val = expressionRule var = lValRule #
	actionGetStringrefSoundAction
	| ACTION_GET_STRREF_FS val = expressionRule var = lValRule #
	actionGetStringrefFemaleSoundAction
	| DECOMPRESS_BIFF files += fileRule+ # decompressBiffAction
	| DEFINE_ACTION_MACRO name = lValRule BEGIN declarations +=
	macroLocalDeclarationRule* actions += tp2ActionRule* END #
	defineActionMacroAction
	| DEFINE_PATCH_MACRO name = lValRule BEGIN declarations +=
	macroLocalDeclarationRule* actions += patchActionRule* END #
	definePatchMacroAction
	| DEFINE_ACTION_FUNCTION name = lValRule
	(
		INT_VAR intVars += varAssignmentTupleRule*
	)?
	(
		STR_VAR strVars += varAssignmentTupleRule*
	)?
	(
		RET retVars += lValRule*
	)?
	(
		RET_ARRAY retArrays += lValRule*
	)? BEGIN actions += tp2ActionRule* END # defineActionFunctionAction
	| DEFINE_PATCH_FUNCTION name = lValRule
	(
		INT_VAR intVars += varAssignmentTupleRule*
	)?
	(
		STR_VAR strVars += varAssignmentTupleRule*
	)?
	(
		RET retVars += lValRule*
	)?
	(
		RET_ARRAY retArrays += lValRule*
	)? BEGIN patch += patchActionRule* END # definePatchFunctionAction
	| DEFINE_DIMORPHIC_FUNCTION name = lValRule
	(
		INT_VAR intVars += varAssignmentTupleRule*
	)?
	(
		STR_VAR strVars += varAssignmentTupleRule*
	)?
	(
		RET retVars += lValRule*
	)?
	(
		RET_ARRAY retArrays += lValRule*
	)? BEGIN actions += tp2ActionRule* END # defineDimorphicFunctionAction
	| LAUNCH_ACTION_MACRO name = lValRule # launchActionMacroAction
	| LAUNCH_ACTION_FUNCTION functionName = lValRule
	(
		INT_VAR intVars += functionCallVarAssignmentTupleRule*
	)?
	(
		STR_VAR strVars += functionCallStrVarAssignmentTupleRule*
	)?
	(
		RET retVars += functionCallVarAssignmentTupleRule*
	)?
	(
		RET_ARRAY retArrayVars += functionCallVarAssignmentTupleRule*
	)? END # launchActionFunctionAction
	| ACTION_TIME var = lValRule BEGIN actions += tp2ActionRule* END #
	actionTimeAction
;

varAssignmentRule
:
	var = lValRule EQ val = expressionRule # assignVar
	| var = lValRule ASSIGN_ADD val = expressionRule # addAssignVar
	| var = lValRule ASSIGN_SUB val = expressionRule # subAssignVar
	| var = lValRule ASSIGN_MULT val = expressionRule # multAssignVar
	| var = lValRule ASSIGN_DIV val = expressionRule # divAssignVar
	| var = lValRule ASSIGN_BAND val = expressionRule # bandAssignVar
	| var = lValRule ASSIGN_BOR val = expressionRule # borAssignVar
	| var = lValRule ASSIGN_BLSR val = expressionRule # blsrAssignVar
	| var = lValRule ASSIGN_BLSL val = expressionRule # blslAssignVar
	| PLUS PLUS var = lValRule # incrementAssignVar
	// this matches '+ + i', but defining '++' token conflicts with replyTransition rule in case of merged grammar

	| MINUSMINUS var = lValRule # decrementAssignVar
;

functionCallStrVarAssignmentTupleRule
:
	name = lValRule
	(
		EQ evaluateBuffer = EVALUATE_BUFFER? val = expressionRule
	)?
;

functionCallVarAssignmentTupleRule
:
	name = lValRule
	(
		EQ val = expressionRule
	)?
;

varAssignmentTupleRule
:
	name = lValRule EQ val = expressionRule
;

arrayTupleRule
:
	keys += lValRule
	(
		COMMA keys += lValRule
	)* FAT_ARROW val = expressionRule
;

stringSetTupleRule
:
	stringToReplace = expressionRule newValue = traLineRule
;

copyKitTupleRule
:
	key = stringRule val = stringRule
;

// TODO: rules for match branches are almost identical and should be unified somehow

patchMatchBranchRule
:
	guardValues += expressionRule+
	(
		WHEN condition = expressionRule
	)? BEGIN actions += patchActionRule* END # specificPatchMatchBranch
	| ANY condition = expressionRule BEGIN actions += patchActionRule* END #
	anyPatchMatchBranch
;

matchBranchRule
:
	guardValues += expressionRule+
	(
		WHEN condition = expressionRule
	)? BEGIN actions += tp2ActionRule* END # specificMatchBranch
	| ANY condition = expressionRule BEGIN actions += tp2ActionRule* END #
	anyMatchBranch
;

moveRegexFileExpressionRule
:
	PAREN_OPEN from = directoryFileRegexpRule PAREN_CLOSE toFile = fileRule
;

directoryFileRegexpRule
:
	fromDirectory = fileRule EVALUATE_REGEXP? regexpExpr = regexpRule #
	defaultDirectoryFileRegexp
	| fromDirectory = fileRule EXACT_MATCH fileName = stringRule #
	exactMatchDirectoryFileRegexp
;

patchActionRule
:
	PATCH # patchPlacholder
	| SAY offset = offsetRule text = dlgLineRule # sayAction
	| PATCH_FAIL displayMessage = traLineRule # patchFailAction
	| PATCH_ABORT displayMessage = traLineRule # patchAbortAction
	| PATCH_WARN displayMessage = traLineRule # patchWarnAction
	| PATCH_PRINT displayMessage = traLineRule # patchPrintAction
	| PATCH_LOG displayMessage = traLineRule # patchLogAction
	| SAY_EVALUATED offset = offsetRule string = stringRule # sayEvaluatedAction
	| TO_UPPER var = lValRule # toUpperAction
	| TO_LOWER var = lValRule # toLowerAction
	| TEXT_SPRINT var = lValRule string = expressionRule # textSprintAction
	| SPRINT var = lValRule string = exprOrRefRule # sprintAction
	| SNPRINT condition = expressionRule var = lValRule string = stringRule #
	snprintAction
	| SPRINTF var = lValRule string = stringRule PAREN_OPEN formatVals +=
	expressionRule* PAREN_CLOSE # sprintfAction
	| SOURCE_BIFF var = lValRule file = fileRule # sourceBiffAction
	| SPACES var = lValRule text = stringRule # spacesAction
	| QUOTE var = lValRule text = stringRule # quoteAction
	| REPLACE case = optCaseRule exact = optExactRule regexp = regexpRule text =
	dlgLineRule # replaceAction
	| REPLACE_TEXTUALLY case = optCaseRule exact = optExactRule regexp =
	regexpRule string = stringRule
	(
		PAREN_OPEN size = expressionRule PAREN_CLOSE
	)? # replaceTextuallyAction
	| EVALUATE_BUFFER_SPECIAL string = stringRule # evaluateBufferSpecialAction
	| EVALUATE_BUFFER # evaluateBufferAction
	| APPLY_BCS_PATCH patchFile = fileRule # applyBcsPatchAction
	| APPLY_BCS_PATCH_OR_COPY patchFile = fileRule copyFile = fileRule #
	applyBcsPatchOrCopyAction
	| WRITE_BYTE offset = offsetRule value = expressionRule # writeByteAction
	| WRITE_SHORT offset = offsetRule value = expressionRule # writeShortAction
	| WRITE_LONG offset = offsetRule value = expressionRule # writeLongAction
	| WRITE_ASCII offset = offsetRule string = expressionRule requiredSize =
	sharpNumberOrExpressionInParensRule? # writeAsciiAction
	| WRITE_ASCII_LIST offset = offsetRule strings += expressionRule* #
	writeAsciiListAction
	| WRITE_ASCII_TERMINATE offset = offsetRule string = expressionRule #
	writeAsciiTerminateAction
	| WRITE_EVALUATED_ASCII offset = offsetRule string = expressionRule
	requiredSize = sharpNumberOrExpressionInParensRule? #
	writeEvaluatedAsciiAction
	| WRITE_FILE offset = offsetRule file = fileRule # writeFileAction
	| INSERT_FILE offset = offsetRule file = fileRule # insertFileAction
	| APPEND_FILE
	(
		text = TEXT
	)? file = fileRule # appendFileAction
	| APPEND_FILE_EVALUATE
	(
		text = TEXT
	)? file = fileRule # appendFileEvaluateAction
	| REPLACE_BCS_BLOCK evaluateBuffer = EVALUATE_BUFFER? case = optCaseRule
	oldFile = fileRule newFile = fileRule
	(
		ON_MISMATCH patch += patchActionRule* END
	)? # replaceBcsBlockAction
	| INSERT_BYTES offset = offsetRule value = expressionRule # insertBytesAction
	| DELETE_BYTES offset = offsetRule value = expressionRule # deleteBytesAction
	| READ_BYTE offset = offsetRule var = lValRule
	(
		ELSE defaultValue = expressionRule
	)? # readByteAction
	| READ_SBYTE offset = offsetRule var = lValRule
	(
		ELSE defaultValue = expressionRule
	)? # readSbyteAction
	| READ_SHORT offset = offsetRule var = lValRule
	(
		ELSE defaultValue = expressionRule
	)? # readShortAction
	| READ_SSHORT offset = offsetRule var = lValRule
	(
		ELSE defaultValue = expressionRule
	)? # readSshortAction
	| READ_LONG offset = offsetRule var = lValRule
	(
		ELSE defaultValue = expressionRule
	)? # readLongAction
	| READ_SLONG offset = offsetRule var = lValRule
	(
		ELSE defaultValue = expressionRule
	)? # readSlongAction
	| READ_ASCII offset = offsetRule var = lValRule
	(
		ELSE defaultValue = stringRule
	)?
	(
		PAREN_OPEN size = expressionRule PAREN_CLOSE
		(
			ignoreTrailingNulls = NULL
		)?
	)? # readAsciiAction
	| READ_STRREF offset = offsetRule var = lValRule
	(
		ELSE defaultValue = stringRule
	)? # readStrRefAction
	| READ_STRREF_F offset = offsetRule var = lValRule
	(
		ELSE defaultValue = stringRule
	)? # readStrRefFAction
	| READ_STRREF_S offset = offsetRule var = lValRule
	(
		ELSE defaultValue = stringRule
	)? # readStrRefSAction
	| READ_STRREF_FS offset = offsetRule var = lValRule
	(
		ELSE defaultValue = stringRule
	)? # readStrRefFSAction
	| GET_OFFSET_ARRAY var = lValRule
	(
		(
			values += expressionRule
			(
				values += expressionRule
			)+
		)
		|
		(
			predefinedSetName = lValRule
		)
	) # getOffsetArrayAction
	| GET_OFFSET_ARRAY2 var = lValRule
	(
		(
			values += expressionRule
			(
				values += expressionRule
			)+
		)
		|
		(
			predefinedSetName = lValRule
		)
	) # getOffsetArray2Action
	| DEFINE_ASSOCIATIVE_ARRAY var = lValRule BEGIN vals += arrayTupleRule* END #
	defineAssociativeArrayAction
	| GET_STRREF val = expressionRule var = lValRule # getStrRefAction
	| GET_STRREF_F val = expressionRule var = lValRule # getStrRefFAction
	| GET_STRREF_S val = expressionRule var = lValRule # getStrRefSAction
	| GET_STRREF_FS val = expressionRule var = lValRule # getStrRefFSAction
	|
	(
		SET
	)? assignment = varAssignmentRule # setAction
	| WHILE condition = expressionRule BEGIN patch += patchActionRule* END #
	whileAction
	| FOR PAREN_OPEN initActions += patchActionRule* SEMICOLON condition =
	expressionRule SEMICOLON incrementActions += patchActionRule* PAREN_CLOSE
	BEGIN bodyActions += patchActionRule* END # forAction
	| PATCH_BASH_FOR selector = directoryFileRegexpRule BEGIN actions +=
	patchActionRule* END # patchBashForAction
	| DEFINE_ARRAY var = lValRule BEGIN elements += expressionRule* END #
	defineArrayAction
	| SORT_ARRAY_INDICES var = lValRule
	(
		lexicographically = LEXICOGRAPHICALLY
		| numerically = NUMERICALLY
	)? # sortArrayIndicesAction
	| PATCH_FOR_EACH var = lValRule IN vals += expressionRule* BEGIN actions +=
	patchActionRule* END # patchForEachAction
	| PHP_EACH val = expressionRule
	/* actually weidu accepts a list of pairs, but the purpose is unclear */
	AS keyTemplate = lValRule FAT_ARROW valTemplate = lValRule BEGIN actions +=
	patchActionRule* END # phpEachAction
	| CLEAR_ARRAY var = lValRule # clearArrayAction
	| PATCH_IF val = expressionRule
	(
		THEN
	)? BEGIN ifActions += patchActionRule* END
	(
		(
			ELSE elseActions += patchActionRule
		)
		|
		(
			ELSE BEGIN elseActions += patchActionRule* END
		)
	)? # patchIfAction
	| PATCH_MATCH val = expressionRule WITH branches += patchMatchBranchRule*
	DEFAULT defaultActions += patchActionRule* END # patchMatchAction
	| PATCH_TRY actions += patchActionRule* WITH branches += patchMatchBranchRule*
	DEFAULT defaultActions += patchActionRule* END # patchTryAction
	| PATCH_RERAISE # patchReraiseAction
	| PATCH_INCLUDE files += fileRule+ # patchIncludeAction
	| PATCH_REINCLUDE files += fileRule+ # patchReincludeAction
	| PATCH_WITH_TRA tras += fileRule* BEGIN actions += patchActionRule* END #
	patchWithTraAction
	| PATCH_WITH_SCOPE BEGIN actions += patchActionRule* END #
	patchWithScopeAction
	| SET_2DA_ENTRY row = expressionRule col = expressionRule requiredColCount =
	expressionRule val = expressionRule # set2daEntryAction
	| SET_2DA_ENTRY_LATER template = stringRule row = expressionRule col =
	expressionRule val = expressionRule # set2daEntryLaterAction
	| SET_2DA_ENTRIES_NOW template = stringRule requiredColCount =
	expressionRule # set2daEntriesNowAction
	| PRETTY_PRINT_2DA row = expressionRule? # prettyPrint2daAction
	| INSERT_2DA_ROW afterRow = expressionRule requiredColCount = expressionRule
	text = expressionRule # insert2daRowAction
	| REMOVE_2DA_ROW row = expressionRule requiredColCount = expressionRule #
	remove2daRowAction
	| PATCH_READLN var = lValRule # patchReadlnAction
	| PATCH_RANDOM_SEED val = expressionRule # patchRandomSeedAction
	| ADD_STORE_ITEM overrideExistingInstance = PLUS? itemName = expressionRule
	position = addStoreItemPositionRule? charge1 =
	sharpNumberOrExpressionInParensRule charge2 =
	sharpNumberOrExpressionInParensRule charge3 =
	sharpNumberOrExpressionInParensRule flags = expressionRule stack =
	sharpNumberOrExpressionInParensRule unlimited = expressionRule? #
	addStoreItemAction
	| REMOVE_STORE_ITEM itemNames += expressionRule+ # removeStoreItemAction
	| READ_2DA_ENTRY row = expressionRule col = expressionRule requiredColCount =
	expressionRule var = lValRule # read2daEntryAction
	| READ_2DA_ENTRIES_NOW var = lValRule requiredColCount = expressionRule #
	read2daEntriesNowAction
	| READ_2DA_ENTRY_FORMER template = stringRule row = expressionRule col =
	expressionRule var = stringRule # read2daEntryFormerAction
	| COUNT_2DA_ROWS requiredColumnCount = expressionRule var = lValRule #
	count2daRowsAction
	| COUNT_2DA_COLS var = lValRule # count2daColsAction
	| COUNT_REGEXP_INSTANCES case = optCaseRule exact = optExactRule regexp =
	regexpRule var = lValRule # countRegexpInstancesAction
	| LOOKUP_IDS_SYMBOL_OF_INT var = lValRule file = fileRule val =
	expressionRule # lookupIdsSymbolOfIntAction
	| COMPILE_BAF_TO_BCS # compileBafToBcsAction
	| DECOMPILE_BCS_TO_BAF # decompileBcsToBafAction
	| DECOMPILE_DLG_TO_D # decompileDlgToDAction
	| COMPILE_D_TO_DLG # compileDToDlgAction
	| DECOMPILE_AND_PATCH BEGIN patchActions += patchActionRule* END #
	decompileAndPatchAction
	| REFACTOR_TRIGGER case = optCaseRule exact = optExactRule regexp = regexpRule
	string = stringRule # refactorTriggerAction
	| REPLACE_EVALUATE case = optCaseRule findRegexp = regexpRule BEGIN actions +=
	patchActionRule* END replaceRegexp = regexpRule # replaceEvaluateAction
	| ADD_GAM_NPC npcCRE = expressionRule npcARE = expressionRule xCoord =
	sharpNumberOrExpressionInParensRule yCoord =
	sharpNumberOrExpressionInParensRule # addGamNpcAction
	| ADD_MAP_NOTE xCoord = sharpNumberOrExpressionInParensRule yCoord =
	sharpNumberOrExpressionInParensRule color = stringRule note = exprOrRefRule #
	addMapNoteAction
	| ADD_KNOWN_SPELL splName = expressionRule spellLevel =
	sharpNumberOrExpressionInParensRule spellType = expressionRule #
	addKnownSpellAction
	| ADD_MEMORIZED_SPELL splName = expressionRule spellLevel =
	sharpNumberOrExpressionInParensRule spellType = expressionRule
	(
		PAREN_OPEN count = expressionRule PAREN_CLOSE
	)? # addMemorizedSpellAction
	| REMOVE_KNOWN_SPELL splNames += expressionRule* # removeKnownSpellAction
	| REMOVE_MEMORIZED_SPELL splNames += expressionRule* #
	removeMemorizedSpellAction
	| SET_BG2_PROFICIENCY proficiency = expressionRule value = expressionRule #
	setBg2ProficiencyAction
	|
	(
		add = ADD_CRE_ITEM
		| replace = REPLACE_CRE_ITEM
	) itmName = expressionRule charge1 = sharpNumberOrExpressionInParensRule
	charge2 = sharpNumberOrExpressionInParensRule charge3 =
	sharpNumberOrExpressionInParensRule flags = expressionRule slot = stringRule
	equip = EQUIP? twoHanded = TWOHANDED? keepAlreadyEquippedItems = NOMOVE? #
	addOrReplaceCreItemAction
	| REMOVE_CRE_ITEM itmNames += expressionRule* # removeCreItemAction
	| REMOVE_CRE_ITEMS # removeCreItemsAction
	| REMOVE_CRE_EFFECTS # removeCreEffectsAction
	| REMOVE_KNOWN_SPELLS # removeKnownSpellsAction
	| REMOVE_MEMORIZED_SPELLS # removeMemorizedSpellsAction
	| PATCH_SILENT # patchSilentAction
	| PATCH_VERBOSE # patchVerboseAction
	| INNER_PATCH buffString = expressionRule BEGIN patchActions +=
	patchActionRule* END # innerPatchAction
	| INNER_PATCH_SAVE var = lValRule buffString = expressionRule BEGIN
	patchActions += patchActionRule* END # innerPatchSaveAction
	| INNER_PATCH_FILE file = fileRule BEGIN patchActions += patchActionRule*
	END # innerPatchFileAction
	| INNER_ACTION BEGIN actions += tp2ActionRule* END # innerActionAction
	| EDIT_SAV_FILE compressionLevel = expressionRule addIfMissing =
	ADD_IF_MISSING? files += fileRule* BEGIN patchActions += patchActionRule*
	END # editSavFileAction
	| DECOMPRESS_REPLACE_FILE start = expressionRule length = expressionRule
	uncompressedLength = expressionRule # decompressReplaceFileAction
	| DECOMPRESS_INTO_FILE start = expressionRule length = expressionRule
	uncompressedLength = expressionRule overwriteFrom = expressionRule overwriteTo
	= expressionRule # decompressIntoFileAction
	| DECOMPRESS_INTO_VAR start = expressionRule length = expressionRule
	uncompressedLength = expressionRule var = lValRule # decompressIntoVarAction
	| COMPRESS_REPLACE_FILE start = expressionRule length = expressionRule
	compressionLevel = expressionRule # compressReplaceFileAction
	| COMPRESS_INTO_FILE start = expressionRule length = expressionRule
	compressionLevel = expressionRule overwriteFrom = expressionRule overwriteTo =
	expressionRule # compressIntoFileAction
	| COMPRESS_INTO_VAR start = expressionRule length = expressionRule
	compressionLevel = expressionRule var = lValRule # compressIntoVarAction
	| LAUNCH_PATCH_MACRO macroName = expressionRule # launchPatchMacroAction
	| LAUNCH_PATCH_FUNCTION functionName = expressionRule
	(
		INT_VAR intVars += functionCallVarAssignmentTupleRule*
	)?
	(
		STR_VAR strVars += functionCallStrVarAssignmentTupleRule*
	)?
	(
		RET retVars += functionCallVarAssignmentTupleRule*
	)?
	(
		RET_ARRAY retArrays += functionCallVarAssignmentTupleRule*
	)? END # launchPatchFunctionAction
	| PATCH_TIME label = expressionRule BEGIN patchActions += patchActionRule*
	END # patchTimeAction
;

macroLocalDeclarationRule
:
	LOCAL_SET assignment = varAssignmentRule # localSetMacroDeclarationRule
	| LOCAL_SPRINT var = lValRule val = exprOrRefRule #
	localSprintMacroDeclarationRule
	| LOCAL_TEXT_SPRINT var = lValRule val = expressionRule #
	localTextSprintMacroDeclarationRule
;

sharpNumberOrExpressionInParensRule
:
	directVal = sharpNumberRule
	| PAREN_OPEN exprVal = expressionRule PAREN_CLOSE
;

addStoreItemPositionRule
:
	AFTER position = expressionRule
	| BEFORE position = expressionRule
	| LAST
	| FIRST
	| AT position = expressionRule
;

offsetRule
:
	expressionRule
;

whenConditionRule
:
	IF_SIZE_IS expr = expressionRule # ifSizeIsWhenCondition
	| IF regexpExpr = regexpRule # ifWhenCondition
	| UNLESS regexpExpr = regexpRule # unlessWhenCondition
	| BUT_ONLY_IF_IT_CHANGES # butOnlyIfItChangesWhenCondition
	| IF_EXISTS # ifExistsWhenCondition
;

optNoBackupRule
:
	PLUS
	| MINUS
;

optGlobRule
:
	GLOB
	| NOGLOB
;

regexpFileTupleRule
:
	fromRegexp = regexpRule toFile = fileRule
;

fromToFilePairRule
:
	fromFile = fileRule toFile = fileRule
;

fileListRule
:
	PAREN_OPEN files += fileRule+ PAREN_CLOSE
;

// last arg is defaultLanguageTRA, does it need a separate production?

languageRule
:
	LANGUAGE name = stringRule directory = stringRule defaultTras += stringRule*
;

//FIXME: component name seem not to be a string, but a @ref

componentRule
:
	BEGIN name = dlgLineRule flags += componentFlagRule* actions += tp2ActionRule*
;

componentFlagRule // TODO

:
	DEPRECATED displayMessage = traLineRule # deprecatedFlag
	| REQUIRE_COMPONENT requiredMod = stringRule requiredComponent = stringRule
	displayMessage = traLineRule # requireComponentFlag
	| FORBID_COMPONENT forbiddenMod = stringRule forbiddenComponent = stringRule
	displayMessage = traLineRule # forbidComponentFlag
	| REQUIRE_PREDICATE predicate = expressionRule displayMessage = traLineRule #
	requirePredicateFlag
	| SUBCOMPONENT name = traLineRule displayCondition = expressionRule? #
	subcomponentFlag
	| FORCED_SUBCOMPONENT name = traLineRule displayCondition = expressionRule? #
	forcedSubcomponentFlag
	| GROUP name = traLineRule displayCondition = expressionRule? # group
	| INSTALL_BY_DEFAULT # installByDefaultFlag
	| DESIGNATED id = numberRule # designatedFlag
	| LABEL id = stringRule # labelFlag
	| METADATA value = stringRule # metadataFlag
	| NO_LOG_RECORD # noLogRecordFlag
;

expressionRule // TODO known as "value" in weidu documentation // FIXME: accidental keyword merging should be somehow handled

:
	val = stringRule # constantStringExpression
	| PAREN_OPEN body = expressionRule PAREN_CLOSE # parensExpression

	// infix expressions 

	| left = expressionRule EXP right = expressionRule # expExpression
	| left = expressionRule MODULO right = expressionRule # moduloExpression
	| left = expressionRule MULT right = expressionRule # productExpression
	| left = expressionRule DIV right = expressionRule # divExpression
	| left = expressionRule PLUS right = expressionRule # sumExpression
	| left = expressionRule MINUS right = expressionRule # subExpression
	| left = expressionRule
	(
		EQ
		| EQEQ
	) right = expressionRule # eqExpression
	| left = expressionRule NEQ right = expressionRule # neqExpression
	| left = expressionRule OR right = expressionRule # orExpression
	| left = expressionRule AND right = expressionRule # andExpression
	| left = expressionRule BAND right = expressionRule # binaryAndExpression
	| left = expressionRule BOR right = expressionRule # binaryOrExpression
	| left = expressionRule BXOR right = expressionRule # binaryXorExpression
	| left = expressionRule BLSL right = expressionRule #
	binaryLeftShiftExpression
	| left = expressionRule BLSR right = expressionRule #
	binaryRightShiftExpression
	| left = expressionRule GT right = expressionRule # gtExpression
	| left = expressionRule GTE right = expressionRule # gteExpression
	| left = expressionRule LT right = expressionRule # ltExpression
	| left = expressionRule LTE right = expressionRule # lteExpression
	| left = expressionRule STRING_COMPARE right = expressionRule #
	strCompareExpression
	| left = expressionRule STRING_COMPARE_CASE right = expressionRule #
	strCompareCaseExpression
	| left = expressionRule STRING_EQUAL right = expressionRule # strEqExpression
	| left = expressionRule STRING_EQUAL_CASE right = expressionRule #
	strEqCaseExpression
	| left = expressionRule STRING_MATCHES_REGEXP right = expressionRule #
	strMatchesExpression
	| left = expressionRule STRING_CONTAINS_REGEXP right = expressionRule #
	strContainsExpression

	// prefix expressions 

	| BYTE_AT operand = expressionRule # byteAtExpression
	| NOT operand = expressionRule # notExpression
	| SBYTE_AT operand = expressionRule # sbyteAtExpression
	| SHORT_AT operand = expressionRule # shortAtExpression
	| SSHORT_AT operand = expressionRule # sshortAtExpression
	| LONG_AT operand = expressionRule # longAtExpression
	| SLONG_AT operand = expressionRule # slongAtExpression
	| ABS operand = expressionRule # absExpression
	| BNOT operand = expressionRule # bnotExpression
	| GAME_IS operand = expressionRule # gameIsExpression
	| ENGINE_IS operand = expressionRule # engineIsExpression
	| GAME_INCLUDES operand = expressionRule # gameIncludesExpression
	| VARIABLE_IS_SET operand = expressionRule # variableIsSetExpression
	| IS_AN_INT operand = expressionRule # isAnIntExpression

	//

	| IDS_OF_SYMBOL PAREN_OPEN file = fileRule symbol = stringRule PAREN_CLOSE #
	idsOfSymbolExpression
	| left = expressionRule EXP PAREN_OPEN expressionRule expressionRule
	PAREN_CLOSE # fractionalExpExpression // FIXME: this is a weird case that does not fit into "infix" rule, should write all expressions as a separate rules instead

	| expressionRule QUESTION_MARK expressionRule COLON expressionRule #
	ternaryExpression
	| TRA_ENTRY_EXISTS PAREN_OPEN entry = stringRule tras = stringRule*
	PAREN_CLOSE # traEntryExistsExpression
	| IS_SILENT # isSilentExpression
	| MOD_IS_INSTALLED modName = expressionRule modComponent = expressionRule #
	modIsInstalledExpression
	| INSTALL_ORDER modName1 = stringRule modComponent1 = stringRule
	(
		AFTER
		| BEFORE
	) modName2 = stringRule modComponent2 = stringRule # installOrderExpression
	| ID_OF_LABEL modName = stringRule label = stringRule # idOfLabelExpression
	| STATE_WHICH_SAYS dlgLineRule FROM stringRule # stateWhichSaysExpression
	| STATE_WHICH_SAYS expressionRule IN tra = stringRule FROM stringRule #
	stateWhichSaysInExpression
	| RESOLVE_STR_REF PAREN_OPEN dlgLineRule PAREN_CLOSE # resolveStrRefExpression
	| NEXT_STRREF # nextStrRefExpression
	| RANDOM PAREN_OPEN lowerBound = expressionRule upperBound = expressionRule
	PAREN_CLOSE # randomExpression
	| BUFFER_LENGTH # bufferLengthExpression
	| INDEX PAREN_OPEN case = optCaseRule exact = optExactRule stringRule
	stringRule expressionRule? PAREN_CLOSE # indexExpression
	| RINDEX PAREN_OPEN case = optCaseRule exact = optExactRule stringRule
	stringRule expressionRule? PAREN_CLOSE # rindexExpression
	| INDEX_BUFFER PAREN_OPEN case = optCaseRule exact = optExactRule regexp =
	regexpRule startFrom = expressionRule? PAREN_CLOSE # indexBufferExpression
	| RINDEX_BUFFER PAREN_OPEN case = optCaseRule exact = optExactRule regexp =
	stringRule startFrom = expressionRule? PAREN_CLOSE # rindexBufferExpression
	| STRING_LENGTH expr = expressionRule # stringLengthExpression
	| FILE_CONTAINS fileName = fileRule regexStr = regexpRule #
	fileContainsExpression
	| FILE_CONTAINS_EVALUATED PAREN_OPEN fileName = fileRule regexStr = regexpRule
	PAREN_CLOSE # fileContainsEvaluatedExpression
	| RESOURCE_CONTAINS fileName = stringRule varsRegexp = stringRule #
	resourceContainsExpression
	| FILE_EXISTS fileName = stringRule # fileExistsExpression
	| DIRECTORY_EXISTS directoryName = expressionRule # directoryExistsExpression
	| FILE_EXISTS_IN_GAME fileName = expressionRule # fileExistsInGameExpression
	| FILE_MD5 fileName = stringRule md5 = expressionRule # fileMd5Expression
	| FILE_IS_IN_COMPRESSED_BIFF fileName = expressionRule #
	fileIsInCompressedBiffExpression
	| BIFF_IS_COMPRESSED fileName = expressionRule # biffIsCompressedExpression
	| FILE_SIZE fileName = expressionRule expectedFileSize = expressionRule #
	fileSizeExpression
	| SIZE_OF_FILE fileName = expressionRule # sizeOfFileExpression
	| VALID_SCRIPT_ACTIONS expressionRule # validScriptActionsExpression
	| VALID_SCRIPT_TRIGGERS expressionRule # validScriptTriggersExpression
	//	| PERCENT variable PERCENT # variableEvaluationExpression // it seems that syntactically vars subset of strings ie  var ~%a%~ is a proper string 
	// or may be not, apparently SET x = a + b is valid

	| EVALUATE_BUFFER expressionRule # evaluateBufferExpression
	| mapAccessor = mapAccessorRule # getMapElementExpression
	// %WEIDU_ARCH%	- variable
	// %WEIDU_OS% - variable
	// 	%COMPONENT_NUMBER% - variable
	// 	%INTERACTIVE% - variable
	// NAME1 - unclear
	// NAME2 - unclear
	// UNIDENTIFIED_DESC - unclear
	// IDENTIFIED_DESC - unclear
	// BIO - unclear
	// Almost everything in SNDSLOT.IDS or SOUNDOFF.IDS works as well - unclear

;

mapAccessorRule
:
	DOLLAR mapExpression = lValRule PAREN_OPEN keys += expressionRule* PAREN_CLOSE
;

lValRule
:
	var = stringRule # variableLval
	| EVALUATE_BUFFER var = lValRule # evalVariableLval
	| var = mapAccessorRule # mapLval
;

optCaseRule
:
	| CASE_SENSITIVE
	| CASE_INSENSITIVE
;

optExactRule
:
	| EXACT_MATCH
	| EVALUATE_REGEXP
;

regexpRule
:
	stringRule // TODO: not a string

;

numberRule
:
	stringLiteralRule
;

exprOrRefRule
:
	expressionRule
	| referenceRule
;
