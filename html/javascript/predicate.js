// used to evaluate predicate expressions.
"use strict";

// Parses a predicate expression and returns an array:
// 'false'				false
// 'true'				true
// [0-9]+			Number
// '[^'\n\r]+'		String
// "[^"\n\r]+"		String
// +, -, etc			{type: 'operator', value: '+'}
// foo.bar			{type: 'member', target: 'foo', member: 'bar'}
// len, and, etc		{type: 'keyword', value: 'and'}
function parse_predicate(expr)
{
	var parts = [
		' +',				// whitespace
		'true',			// boolean
		'false',			// boolean
		'[0-9]+',		// Number
		"'[^'\n\r]*'",	// String
		'"[^"\n\r]*"',	// String
		'\\+|-|\\*|/|\\%|==|!=|<|<=|>=|>'	// operator
	];
	parts = parts.map(function (x) {return '(' + x + ')';});
	var re = new RegExp(parts.join('|'), "gm");	// unfortunately there is no verbose option
	
	var result = [];
	var match;
	while ((match = re.exec(expr)))
	{
		// match a token
		if (match[2])
			result.push(true);
		else if (match[3])
			result.push(false);
		else if (match[4])
			result.push(parseInt(match[4], 10));
		else if (match[5])
			result.push(match[5].slice(1, match[5].length - 1));
		else if (match[6])
			result.push(match[6].slice(1, match[6].length - 1));
		else if (match[7])
			result.push({type: 'operator', value: match[7]});
		else
			throw SyntaxError("failed to parse '{0}'".format(expr));
			
		if (re.lastIndex == expr.length)
			return result;
		
		// if not done, then match spaces
		match = re.exec(expr);
		if (!match || !match[1])
			throw SyntaxError("failed to parse '{0}'".format(expr));
			
		if (re.lastIndex == expr.length)
			return result;
	}
	
	// if not everything was matched then fail
	throw SyntaxError("failed to parse '{0}'".format(expr));
}
 