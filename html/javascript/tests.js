// Javascript unit tests.
"use strict";

// http://api.qunitjs.com/category/assert/
function fails(expr)
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
	
	deepEqual(parse_predicate('"foo"'), ['foo']);
	deepEqual(parse_predicate("'foo bar'"), ['foo bar']);
	
	deepEqual(parse_predicate('len to_str'), [{type: 'unary', value: 'len'}, {type: 'unary', value: 'to_str'}]);
	deepEqual(parse_predicate('+ =='), [{type: 'binary', value: '+'}, {type: 'binary', value: '=='}]);
	deepEqual(parse_predicate("if"), [{type: 'ternary', value: 'if'}]);
	deepEqual(parse_predicate("log concat"), [{type: 'variadic', value: 'log'}, {type: 'variadic', value: 'concat'}]);
	
	deepEqual(parse_predicate("foo.bar"), [{type: 'member', target: 'foo', member: 'bar'}]);
	
	deepEqual(parse_predicate("options.OSPF '10.1.0.1' selection.name == and"), [
		{type: 'member', target: 'options', member: 'OSPF'},
		'10.1.0.1',
		{type: 'member', target: 'selection', member: 'name'},
		{type: 'binary', value: '=='},
		{type: 'binary', value: 'and'}
	]);
	
	fails('foo');
	fails('false *** true');
});
