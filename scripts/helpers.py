# Misc functions that pretty much every Python modeler will need to use.
import json, logging, logging.handlers, subprocess

class Env(object):
	def __init__(self):
		self.options = None	# command line options, verbose is required
		self.config = None		# dictionary, requires server, port, and path entries
		self.logger = None

env = Env()

def ip_to_int(ip):
	parts = ip.split('.')
	if len(parts) != 4:
		raise Exception("expected an IP address but found: '%s'" % ip)
	return (int(parts[0]) << 24) | (int(parts[1]) << 16) | (int(parts[2]) << 8) | int(parts[3])

def int_to_ip(value):
	return '%s.%s.%s.%s' % ((value >> 24) & 0xFF, (value >> 16) & 0xFF, (value >> 8) & 0xFF, value & 0xFF)
	
def add_if_missing(sequence, value):
	if value not in sequence:
		sequence.append(value)

def add_label(data, target, label, key, level = 0, style = ''):
	if label:
		sort_key = '%s-%s' % (level, key)
		if style:
			data['labels'].append({'target-id': target, 'label': label, 'level': level, 'sort-key': sort_key, 'style': style})
		else:
			data['labels'].append({'target-id': target, 'label': label, 'level': level, 'sort-key': sort_key})

def add_gauge(data, target, label, value, level, style, sort_key):
	data['gauges'].append({'entity-id': target, 'label': label, 'value': value, 'level': level, 'style': style, 'sort-key': sort_key})

def add_details(data, target, label, details, opened, sort_key, key):
	data['details'].append({'entity-id': target, 'label': label, 'details': json.dumps(details), 'open': opened, 'sort-key': sort_key, 'id': key})

def add_relation(data, left, right, style = '', left_labels = None, middle_labels = None, right_labels = None, predicate = None):
	relation = {'left-entity-id': left, 'right-entity-id': right, 'style': style}
	if left_labels:
		relation['left-labels'] = left_labels
	if middle_labels:
		relation['middle-labels'] = middle_labels
	if right_labels:
		relation['right-labels'] = right_labels
	if predicate:
		relation['predicate'] = predicate
	data['relations'].append(relation)

def open_alert(data, target, key, mesg, resolution, kind):
	data['alerts'].append({'entity-id': target, 'key': key, 'mesg': mesg, 'resolution': resolution, 'kind': kind})

def close_alert(data, target, key):
	data['alerts'].append({'entity-id': target, 'key': key})

def secs_to_str(secs):
	if secs >= 365.25*86400:
		value = '%.2f' % (secs/(365.25*86400))		# http://en.wikipedia.org/wiki/Month#Month_lengths
		units = 'year'
	elif secs >= 365.25*86400/12:
		value = '%.2f' % (secs/(365.25*86400/12))
		units = 'month'
	elif secs >= 86400:
		value = '%.1f' % (secs/(86400))
		units = 'day'
	elif secs >= 60*60:
		value = '%.1f' % (secs/(60*60))
		units = 'hour'
	elif secs >= 60:
		value = '%.0f' % (secs/(60))
		units = 'minute'
	elif secs >= 1:
		value = '%.0f' % secs
		units = 'seconds'
	else:
		value = '%.3f' % (1000*secs)
		units = 'msec'
	if value == '1':
		return '%s %s' % (value, units)
	else:
		return '%s %ss' % (value, units)

def run_process(command):
	if env.options.verbose >= 4:
		env.logger.debug("running '%s'" % command)
	process = subprocess.Popen(command, bufsize = 8*1024, shell = True, stdout = subprocess.PIPE, stderr = subprocess.PIPE)
	(outData, errData) = process.communicate()
	if process.returncode != 0:
		env.logger.error(errData)
		raise ValueError('return code was %s:' % process.returncode)
	elif env.options.verbose == 4:
		env.logger.debug("   %s lines in result" % outData.count('\n'))
	elif env.options.verbose >= 5:
		env.logger.debug("stdout: '%s'" % outData)
		if errData:
			env.logger.debug("stderr: '%s'" % errData)
	return outData

