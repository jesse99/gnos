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
		' +',				// whitespace (not returned)
		'true',			// Boolean
		'false',			// Boolean
		'[+-]?[0-9]+',	// Number
		"'[^'\n\r]*'",	// String
		'"[^"\n\r]*"',	// String
		'is_empty|is_not_empty|len|not|to_num|to_lower|to_str|to_upper',		// unary operator (Object)
		'\\+|-|\\*|/|\\%|==|!=|<=|>=|<|>|and|or|contains|ends_with|starts_with',	// binary operator (Object)
		'if',																		// ternary operator (Object)
		'concat|log',															// variadic operator (Object)
		'[a-zA-Z_]\\w*\\.[a-zA-Z_]\\w*'										// member (Object)
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
			result.push({type: 'unary', value: match[7]});
		else if (match[8])
			result.push({type: 'binary', value: match[8]});
		else if (match[9])
			result.push({type: 'ternary', value: match[9]});
		else if (match[10])
			result.push({type: 'variadic', value: match[10]});
		else if (match[11])
			result.push({type: 'member', target: match[11].split('.')[0], member: match[11].split('.')[1]});
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

function eval_predicate(context, expr)
{
	var terms = parse_predicate(expr);
	
	var stack = [];
	$.each(terms, function (i, term)
	{
		if (term === false || term === true || $.isNumeric(term))
			stack.push(term);
		else if (typeof(term) == 'string')
			stack.push(term);
		else if ($.isPlainObject(term) && term.type == 'unary')
			eval_unary(stack, term.value);
		else if ($.isPlainObject(term) && term.type == 'binary')
			eval_binary(stack, term.value);
		else if ($.isPlainObject(term) && term.type == 'ternary')
			eval_ternary(stack, term.value);
		else if ($.isPlainObject(term) && term.type == 'variadic')
			eval_variadic(stack, term.value);
		else if ($.isPlainObject(term) && term.type == 'member')
			eval_member(context, stack, term.target, term.member);
		else
			throw EvalError("can't evaluate '{0}'".format(term));
	});
	
	if (stack.length === 0)
		throw EvalError("'{0}' evaluated to nothing".format(expr));
	else if (stack.length > 1)
		throw EvalError("'{0}' evaluated to {1:j}".format(expr, stack));
		
	if (stack[0] !== false && stack[0] !== true)
		throw EvalError("'{0}' evaluated to {1:j}".format(expr, stack[0]));
	
	return stack[0];
}

function pop_any(stack, operator)
{
	if (stack.length === 0)
		throw EvalError("{0}: empty stack".format(operator));
		
	return stack.pop();
}

function pop_string(stack, operator)
{
	var result = pop_any(stack, operator);
	
	if (typeof(result) !== 'string')
		throw EvalError("{0}: expected a string but found {1}".format(operator, result));
		
	return result;
}

function pop_number(stack, operator)
{
	var result = pop_any(stack, operator);
	
	if (!$.isNumeric(result))
		throw EvalError("{0}: expected a number but found {1}".format(operator, result));
		
	return result;
}

function pop_bool(stack, operator)
{
	var result = pop_any(stack, operator);
	
	if (result !== false && result !== true)
		throw EvalError("{0}: expected a bool but found {1}".format(operator, result));
		
	return result;
}

function eval_unary(stack, operator)
{
	switch (operator)
	{
		case 'is_empty':
			var arg = pop_string(stack, operator);
			stack.push(arg.length === 0);
			break;
		
		case 'is_not_empty':
			var arg = pop_string(stack, operator);
			stack.push(arg.length !== 0);
			break;
		
		case 'len':
			var arg = pop_string(stack, operator);
			stack.push(arg.length);
			break;
		
		case 'not':
			var arg = pop_bool(stack, operator);
			stack.push(!arg);
			break;
		
		case 'to_num':
			var arg = pop_string(stack, operator);
			var result = parseInt(arg, 10);
			if (isNaN(result))
				throw EvalError("'to_num failed: '{0}' is not a number".format(arg));
			stack.push(result);
			break;
		
		case 'to_lower':
			var arg = pop_string(stack, operator);
			stack.push(arg.toLowerCase());
			break;
		
		case 'to_str':
			var arg = pop_any(stack, operator);
			stack.push(arg.toString());
			break;
		
		case 'to_upper':
			var arg = pop_string(stack, operator);
			stack.push(arg.toUpperCase());
			break;
		
		default:
			throw EvalError("bad operator: '{0}'".format(operator));
	}
}

function eval_binary(stack, operator)
{
	switch (operator)
	{
		case '+':
			var rhs = pop_number(stack, operator);
			var lhs = pop_number(stack, operator);
			stack.push(lhs + rhs);
			break;
		
		case '-':
			var rhs = pop_number(stack, operator);
			var lhs = pop_number(stack, operator);
			stack.push(lhs - rhs);
			break;
		
		case '*':
			var rhs = pop_number(stack, operator);
			var lhs = pop_number(stack, operator);
			stack.push(lhs * rhs);
			break;
		
		case '%':
			var rhs = pop_number(stack, operator);
			var lhs = pop_number(stack, operator);
			stack.push(lhs % rhs);
			break;
		
		case '==':
			var rhs = pop_any(stack, operator);
			var lhs = pop_any(stack, operator);
			stack.push(lhs === rhs);
			break;
		
		case '!=':
			var rhs = pop_any(stack, operator);
			var lhs = pop_any(stack, operator);
			stack.push(lhs !== rhs);
			break;
		
		case '<=':
			var rhs = pop_any(stack, operator);
			var lhs = pop_any(stack, operator);
			stack.push(lhs <= rhs);
			break;
		
		case '>=':
			var rhs = pop_any(stack, operator);
			var lhs = pop_any(stack, operator);
			stack.push(lhs >= rhs);
			break;
		
		case '<':
			var rhs = pop_any(stack, operator);
			var lhs = pop_any(stack, operator);
			stack.push(lhs < rhs);
			break;
		
		case '>':
			var rhs = pop_any(stack, operator);
			var lhs = pop_any(stack, operator);
			stack.push(lhs >= rhs);
			break;
		
		case 'and':
			var rhs = pop_bool(stack, operator);
			var lhs = pop_bool(stack, operator);
			stack.push(lhs && rhs);
			break;
		
		case 'or':
			var rhs = pop_bool(stack, operator);
			var lhs = pop_bool(stack, operator);
			stack.push(lhs || rhs);
			break;
		
		case 'contains':
			var needle = pop_string(stack, operator);
			var target = pop_string(stack, operator);
			stack.push(needle in target);
			break;
		
		case 'ends_with':
			var suffix = pop_string(stack, operator);
			var target = pop_string(stack, operator);
			stack.push(target.endsWith(suffix));
			break;
		
		case 'starts_with':
			var prefix = pop_string(stack, operator);
			var target = pop_string(stack, operator);
			stack.push(target.startsWith(prefix));
			break;
		
		default:
			throw EvalError("bad operator: '{0}'".format(operator));
	}
}

function eval_ternary(stack, operator)
{
	switch (operator)
	{
		case 'if':
			var false_case = pop_any(stack, operator);
			var true_case = pop_any(stack, operator);
			var predicate = pop_bool(stack, operator);
			if (predicate === true)
				stack.push(true_case);
			else
				stack.push(false_case);
			break;
		
		default:
			throw EvalError("bad operator: '{0}'".format(operator));
	}
}

function eval_variadic(stack, operator)
{
	var count = pop_number(stack, operator);
	
	var args = [];
	for (var i = 0; i < count; ++i)
	{
		args.push(pop_any(stack, operator));
	}
	args.reverse();
	
	switch (operator)
	{
		case 'concat':
			args = args.map(function (x) {return x.toString();});
			stack.push(args.join(''));
			break;
		
		case 'log':
			args = args.map(function (x) {return x.toString();});
			console.log(args.join(''));
			break;
		
		default:
			throw EvalError("bad operator: '{0}'".format(operator));
	}
}

function eval_member(context, stack, target, member)
{
	if (target in context)
	{
		if (member in context[target])
		{
			stack.push(context[target][member]);
		}
		else
		{
			throw EvalError("couldn't find member {0}.{1} in {2:j}".format(target, member, context));
		}
	}
	else
	{
		throw EvalError("couldn't find target {0} in {1:j}".format(target, context));
	}
}

