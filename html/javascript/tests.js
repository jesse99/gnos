// Javascript unit tests.
"use strict";

// http://api.qunitjs.com/category/assert/
function parse_fails(expr)
{
	try
	{
		var result = parse_predicate(expr);
		ok(false, "'{0}' parsed as {1:j}".format(expr, result));
	}
	catch (e)
	{
	}
}

function eval_fails(context, expr)
{
	try
	{
		var result = eval_predicate(context, expr);
		ok(false, "'{0}' evaluated as {1}".format(expr, result));
	}
	catch (e)
	{
	}
}

test("escapeHtml", function()
{
	equal(escapeHtml("hello world"), "hello world");
	equal(escapeHtml(">x<"), "&gt;x&lt;");
});

test("parse_predicate", function()
{
	deepEqual(parse_predicate('false'), [false]);
	deepEqual(parse_predicate('true'), [true]);
	deepEqual(parse_predicate('false   true'), [false, true]);
	
	deepEqual(parse_predicate('42'), [42]);
	deepEqual(parse_predicate('-23'), [-23]);
	
	deepEqual(parse_predicate('"foo"'), ['foo']);
	deepEqual(parse_predicate("'foo bar'"), ['foo bar']);
	
	deepEqual(parse_predicate('len to_str'), [{type: 'unary', value: 'len'}, {type: 'unary', value: 'to_str'}]);
	deepEqual(parse_predicate('+ =='), [{type: 'binary', value: '+'}, {type: 'binary', value: '=='}]);
	deepEqual(parse_predicate("if"), [{type: 'ternary', value: 'if'}]);
	deepEqual(parse_predicate("log concat"), [{type: 'variadic', value: 'log'}, {type: 'variadic', value: 'concat'}]);
	
	deepEqual(parse_predicate("foo.bar"), [{type: 'member', target: 'foo', member: 'bar'}]);
	deepEqual(parse_predicate("foo.bar_bar"), [{type: 'member', target: 'foo', member: 'bar_bar'}]);
	
	deepEqual(parse_predicate("options.OSPF '10.1.0.1' selection.name == and"), [
		{type: 'member', target: 'options', member: 'OSPF'},
		'10.1.0.1',
		{type: 'member', target: 'selection', member: 'name'},
		{type: 'binary', value: '=='},
		{type: 'binary', value: 'and'}
	]);
	
	parse_fails('foo');
	parse_fails('false *** true');
});

test("eval_predicate", function()
{
	var context = {selection: {name: 'blargh', value: 'ful'}};
	
	strictEqual(eval_predicate(context, 'false'), false);
	strictEqual(eval_predicate(context, 'true'), true);
	strictEqual(eval_predicate(context, '"foo" is_empty'), false);
	strictEqual(eval_predicate(context, '"foo" is_not_empty'), true);
	strictEqual(eval_predicate(context, '"foo" is_empty not'), true);
	strictEqual(eval_predicate(context, '"foo" len 3 =='), true);
	strictEqual(eval_predicate(context, '"4" to_num 1 - 3 =='), true);
	strictEqual(eval_predicate(context, '"foo" to_upper "FOO" =='), true);
	strictEqual(eval_predicate(context, '3 100 >'), false);
	
	strictEqual(eval_predicate(context, '"foo" is_empty 23 45 if 45 =='), true);
	strictEqual(eval_predicate(context, '3 "foo" true 3 concat "3footrue" =='), true);
	
	strictEqual(eval_predicate(context, 'selection.value "ful" =='), true);
	
	eval_fails(context, 'false true');
	eval_fails(context, 'false is_empty');
	eval_fails(context, '42');
});
