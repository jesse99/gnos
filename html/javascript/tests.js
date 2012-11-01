// Javascript unit tests.
"use strict";

// http://api.qunitjs.com/category/assert/
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
	
	console.log('pp = {0:j}'.format(parse_predicate('"+ =="')));
	deepEqual(parse_predicate('+ =='), [{type: 'operator', value: '+'}, {type: 'operator', value: '=='}]);

	try
	{
		strictEqual(parse_predicate('false *** true'), null);
		ok(false);
	}
	catch (e)
	{
	}
});
