/*
 * EventQL Grammar
 * Version 1.2.0
 *
 * Copyright (c) the native web GmbH — https://www.thenativeweb.io
 *
 * This grammar file is provided by the native web GmbH. You are granted
 * permission to read and to redistribute this file as part of the EventQL
 * parser project. Modification of this file is not permitted.
 *
 * This file is provided "as is", without warranty of any kind, express or
 * implied, including but not limited to the warranties of merchantability,
 * fitness for a particular purpose, and non-infringement. In no event shall
 * the native web GmbH be liable for any claim, damages, or other liability,
 * whether in an action of contract, tort, or otherwise, arising from, out of,
 * or in connection with this file or the use of this file.
 *
 * For questions, feedback, or any other inquiries, please contact:
 * hello@thenativeweb.io
 */

grammar EventQL;
 options { caseInsensitive = true; }

entrypointQuery: query EOF;										// Main entrypoint
entrypointExpression: expression EOF; 				// Primarily used for testing

query: from+ where? orderBy? groupBy? skip? top? projectInto;

from: FROM variableRef IN source;
source: sourceName | LPAREN query RPAREN;
where: WHERE condition;
orderBy: ORDER BY expression (ASC | DESC)?;
groupBy: GROUP BY expression having?;
having: HAVING condition;
top: TOP expression;
skip: SKIP_KW expression;
projectInto: PROJECT INTO DISTINCT? projectionStructure;

condition: expression;
projectionStructure: expression;

equalityOperator: EQ | NEQ | CONTAINS;
relationalOperator: GT | GTE | LT | LTE;
additiveOperator: PLUS | MINUS;
multiplicativeOperator: ASTERISK | SLASH | MOD;
prefixOperator: NOT | MINUS;

expression:
	atom																						# AtomExpression					// Highest precedence
	| arrayLiteral																	# ArrayExpression
	| mapLiteral																		# MapExpression
	| expression LBRACK expression RBRACK						# ArrayIndexExpression
	| expression DOT unrestrictedIdentifier					# NestedIdentifier
	| functionCall																	# FunctionCallExpression
	| prefixOperator expression											# PrefixExpression
	| LPAREN expression RPAREN											# ParenthesesExpression
	| expression multiplicativeOperator expression	# MultiplicativeExpression
	| expression additiveOperator expression				# AdditiveExpression
	| expression AS IDENTIFIER											# CastExpression
	| expression relationalOperator expression			# RelationalExpression
	| expression equalityOperator expression				# EqualityExpression
	| expression AND expression											# AndExpression
	| expression XOR expression											# XorExpression
	| expression OR expression											# OrExpression 						// Lowest precedence
	;

functionCall:
	functionRef LPAREN (expression (COMMA expression)*)? RPAREN;
mapPair:
	unrestrictedIdentifier COLON expression;
mapLiteral:
	LBRACE (mapPair (COMMA mapPair)*)? RBRACE;
arrayLiteral:
	LBRACK (expression (COMMA expression)*)? RBRACK;

atom: variableRef | INT | FLOAT | STRING | NULL | TRUE | FALSE;
variableRef: IDENTIFIER;
functionRef: IDENTIFIER;
sourceName: IDENTIFIER;
unrestrictedIdentifier: // This is a special case for identifiers that can be keywords (such in keys in a map)
	IDENTIFIER
	| AND
	| ASC
	| AS
	| BY
	| CONTAINS
	| DESC
	| DISTINCT
	| FROM
	| FALSE
	| GROUP
	| HAVING
	| IN
	| INTO
	| TOP
	| NOT
	| NULL
	| OR
	| ORDER
	| PROJECT
	| SKIP_KW
	| TRUE
	| WHERE
	| XOR
	;

// Operators
ASTERISK: '*';
EQ: '==';
GT: '>';
GTE: '>=';
LT: '<';
LTE: '<=';
MINUS: '-';
MOD: '%';
NEQ: '!=';
PLUS: '+';
SLASH: '/';

// Delimiters
COMMA: ',';
COLON: ':';
DOT: '.';
BLOCK_COMMENT_START: '/*';
BLOCK_COMMENT_END: '*/';
BLOCK_COMMENT: BLOCK_COMMENT_START .*? BLOCK_COMMENT_END -> channel(HIDDEN);
LINE_COMMENT: '//' ~[\r\n]* -> channel(HIDDEN);
WHITESPACE: [ \t] -> channel(HIDDEN);
LINE_BREAK: [\r\n] -> channel(HIDDEN);

// Brackets
LBRACE: '{';
LBRACK: '[';
LPAREN: '(';
RBRACE: '}';
RBRACK: ']';
RPAREN: ')';

// Keywords
AND: 'AND';
ASC: 'ASC';
AS: 'AS';
BY: 'BY';
CONTAINS: 'CONTAINS';
DESC: 'DESC';
DISTINCT: 'DISTINCT';
FALSE: 'FALSE';
FROM: 'FROM';
GROUP: 'GROUP';
HAVING: 'HAVING';
IN: 'IN';
INTO: 'INTO';
NOT: 'NOT';
NULL: 'NULL';
OR: 'OR';
ORDER: 'ORDER';
PROJECT: 'PROJECT';
SKIP_KW: 'SKIP'; 	// SKIP is a reserved keyword in ANTLR
TOP: 'TOP';
TRUE: 'TRUE';
WHERE: 'WHERE';
XOR: 'XOR';

ESCAPE_CHARACTER : '\\' ('n' | 'r' | 't' | '"' | '\'' | '\\');

// Identifiers + literals
STRING: '"' (ESCAPE_CHARACTER | ~["\r\n])*? '"' | '\'' (ESCAPE_CHARACTER | ~['\r\n])*? '\'';
FLOAT: [0-9]+ '.' [0-9]+;
INT: [0-9]+;
IDENTIFIER:
	[a-z\u0080-\uFFFF_\p{Emoji}][a-z0-9\u0080-\uFFFF_\p{Emoji}]*;
