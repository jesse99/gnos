# Types used by snmp and linux_ssh to communicate with net-modeler.

class Interface(object):
	def __init__(self):
		self.name = ''				# "eth1"
		self.status = ''				# "up", "down", "dormant", etc
		self.ip = ''					# "10.143.0.254"			may not be set if the device is inactive
		self.net_mask = ''			# "255.255.255.0"			may not be set if the device is inactive
		self.mac_addr = ''			# "00:19:bb:5f:59:8a"		may not be set if the device is inactive
		self.speed = 0.0			# 10000000.0 bps
		self.mtu = ''				# "1500" bytes
		self.in_octets = 0.0			# 9840.0 bytes
		self.out_octets = 0.0		# 9840.0 bytes
		self.last_changed = 0.0	# 2191.0 seconds
	
	# True if the interface is able to communicate.
	@property
	def active(self):
		return self.status == 'up' or self.status == 'dormant'
	
	def __repr__(self):
		return self.ip

class Route(object):
	def __init__(self):
		self.via_ip = ''
		self.dst_subnet = ''
		self.dst_mask = ''
		self.protocol = ''
		self.metric = ''
		self.ifindex = ''				# source interface index
	
	def __repr__(self):
		return '%s via %s' % (self.dst_subnet, self.via_ip)

class Device(object):
	def __init__(self, name, admin_ip, modeler):
		self.__name = name
		self.__admin_ip = admin_ip
		self.__modeler = modeler
		
		self.interfaces = []			# [Interface]
		self.routes = []				# [Route]
		self.system_info = []		#[markdown]
	
	@property
	def name(self):
		return self.__name
	
	@property
	def admin_ip(self):
		return self.__admin_ip
	
	@property
	def modeler(self):
		return self.__modeler
	
	def __repr__(self):
		return self.admin_ip

class Network(object):
	def __init__(self, devices):
		self.__devices = devices
	
	# admin ip => Device
	@property
	def devices(self):
		return self.__devices
	
	# List of all the admin IP addresses in the network.
	@property
	def admin_ips(self):
		return self.__devices.keys()
	
