# Types used by snmp and linux_ssh to communicate with net-modeler.

class Interface(object):
	def __init__(self):
		self.admin_ip = None		# "10.143.0.2"
		self.name = None			# "eth1"
		self.status = None			# "up", "down", "dormant", etc
		self.ip = None				# "10.143.0.254"			may not be set if the device is inactive
		self.index = None			# "2"
		self.net_mask = None		# "255.255.255.0"			may not be set if the device is inactive
		self.mac_addr = None		# "00:19:bb:5f:59:8a"		may not be set if the device is inactive
		self.speed = None			# 10000000.0 bps
		self.mtu = None			# 1500 bytes
		self.in_octets = None		# 9840.0 bytes
		self.out_octets = None		# 9840.0 bytes
		self.last_changed = None	# 2191.0 seconds
	
	# True if the interface is able to communicate.
	@property
	def active(self):
		return self.status == 'up' or self.status == 'dormant'
	
	def __repr__(self):
		return self.ip or '?'

class Route(object):
	def __init__(self):
		self.via_ip = None
		self.dst_subnet = None
		self.dst_mask = None
		self.protocol = None
		self.metric = None
		self.ifindex = None		# source interface index
		
		self.src_interface = None
		self.via_interface = None
		self.dst_admin_ip = None
	
	def __repr__(self):
		return '%s via %s' % (self.dst_subnet or '?', self.via_ip or '?')

class Device(object):
	def __init__(self, config):
		self.__config = config		# from network json
		
		self.uptime = None		# 60.0 seconds
		self.system_info = ''		# "markdown"
		self.interfaces = []			# [Interface]
		self.routes = []				# [Route]
	
	@property
	def config(self):
		return self.__config
	
	@property
	def admin_ip(self):
		return self.__config['ip']
	
	def __repr__(self):
		return self.__config['ip']
