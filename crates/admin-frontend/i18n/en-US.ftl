nav_overview = Overview
nav_guests = Guests
nav_instances = Instances
nav_plans = Plans
nav_nodes = Nodes
nav_nat_leases = NAT Leases
nav_tickets = Tickets
nav_logout = Logout
switch_lang = 中文

# Layout & General
app_title = Cloud Store
admin_console = Admin Console
dash_layout_not_logged_in = Not Logged In
dash_layout_please_login = Please log in to admin account first to access this page.
dash_layout_go_to_login = Go to Login
dash_layout_logout_notice = Logged out of admin session
dash_layout_admin_desc_p1 = The independent admin console maintains the same visual system as the client but with isolated permissions and ports.
store_btn = Store
loading = Loading...
processing = Processing...
submit = Submit
save = Save
cancel = Cancel
edit = Edit
delete = Delete
refresh = Refresh List
status_label = Status
actions_label = Actions

# Login Page
login_admin_title = Admin Login
login_api_base_label = Admin API Base
login_email_label = Email
login_password_label = Password
login_submit_btn = Login and Verify Admin Rights
login_err_not_admin = This account is not an admin and cannot access the admin console.
login_success_notice = Logged in as admin.
login_err_prefix_profile = Failed to get profile: { $err }
login_err_prefix_login = Login failed: { $err }

# Overview Page
overview_title = Admin Dashboard
overview_desc = Manage node inventory, products, guest configs, and ticket status here. The UI shell remains consistent with the customer center, but content and permissions differ.
overview_current_admin = Current Admin

# Node Page
nodes_title = Node Management
nodes_add_btn = Add Node
nodes_add_modal_title = Add New Node
nodes_edit_modal_title = Edit Node: { $name }
nodes_form_name = Node Name
nodes_form_region = Region
nodes_form_cpu = CPU Cores
nodes_form_ram = RAM (MB)
nodes_form_storage = Storage (GB)
nodes_form_api_endpoint = API Endpoint (Optional)
nodes_form_api_endpoint_edit = API Endpoint
nodes_form_incus_token = Incus Trust Token (Optional)
nodes_form_incus_token_edit = API Token
nodes_incus_token_placeholder = token from `incus config trust add <client-name>`
nodes_no_data = No node data found.
nodes_refresh_success = Node list refreshed.
nodes_add_success = Node added successfully.
nodes_update_success = Node updated successfully.
nodes_error_refresh = Refresh failed: { $err }
nodes_error_add = Add failed: { $err }
nodes_error_update = Update failed: { $err }

# Plan Page
plans_title = Plan Management
plans_add_btn = Create New Plan
plans_no_data = No plan data found.
plans_form_id = ID (slug)
plans_form_name = Name
plans_form_desc = Description
plans_form_price = Price (USD/month)
plans_form_cpu = CPU Cores
plans_form_cpu_allowance = CPU Allowance (%)
plans_form_ram = RAM (MB)
plans_form_storage = Storage (GB)
plans_form_bw = Bandwidth (Mbps)
plans_form_traffic = Traffic Limit (GB, 0 for unlimited)
plans_form_active = Is Active
plans_add_modal_title = Create New Plan
plans_edit_modal_title = Edit Plan: { $name }
plans_add_success = Plan created successfully.
plans_update_success = Plan updated successfully.
plans_refresh_success = Plan list refreshed.

# Instance Page
instances_title = Instance Management
instances_search_placeholder = Search Instance ID, User ID...
instances_no_data = No instances found.
instances_table_id = ID
instances_table_node = Node
instances_table_user = User
instances_table_plan = Plan
instances_table_status = Status
instances_table_created = Created At
instances_table_image = OS Template
instances_action_rebuild = Rebuild OS
instances_action_password = Reset Password
instances_rebuild_modal_title = Rebuild Instance OS
instances_rebuild_confirm = Are you sure you want to rebuild instance { $id }? All data will be lost.
instances_password_modal_title = Reset Root Password
instances_password_confirm = Are you sure you want to reset root password for instance { $id }?
instances_rebuild_success = Rebuild request submitted.
instances_password_success = Password reset request submitted.
instances_action_success = Action executed.

# NAT Leases Page
nat_leases_title = NAT Port Leases Management
nat_leases_add_btn = Batch Generate Leases
nat_leases_no_data = No port lease data found.
nat_leases_form_node = Select Node
nat_leases_form_public_ip = Public IP (NAT)
nat_leases_form_start_port = Start Port
nat_leases_form_end_port = End Port
nat_leases_generate_success = Port leases generated successfully.
nat_leases_table_ip = Public IP
nat_leases_table_port = Port
nat_leases_table_target = Target (Instance:Port)
nat_leases_table_status = Status
nat_leases_status_available = Available
nat_leases_status_occupied = Occupied

# Guest Page
guests_title = Guest User Management
guests_search_placeholder = Search Email, User ID...
guests_no_data = No guest data found.
guests_table_id = ID
guests_table_email = Email
guests_table_role = Role
guests_table_balance = Balance
guests_table_created = Created At
guests_action_recharge = Recharge Balance
guests_recharge_modal_title = Recharge for Guest: { $email }
guests_form_amount = Amount (USD)
guests_recharge_success = Recharge successful.

# Ticket Page
tickets_title = Ticket Center
tickets_no_data = No tickets found.
tickets_table_id = ID
tickets_table_user = User
tickets_table_subject = Subject
tickets_table_status = Status
tickets_table_priority = Priority
tickets_action_detail = Details
tickets_detail_modal_title = Ticket Details: { $subject }
tickets_detail_info = Basic Info
tickets_detail_user = User ID: { $id }
tickets_detail_category = Category: { $cat }
tickets_detail_priority = Priority: { $prio }
tickets_detail_status = Status: { $status }
tickets_detail_msg_admin = (Admin)
tickets_detail_msg_user = (User)
tickets_detail_reply_placeholder = Type your reply...
tickets_detail_send_btn = Send Reply
tickets_detail_close_btn = Close Ticket
tickets_reply_success = Reply sent successfully.
tickets_close_success = Ticket closed successfully.

# Dedicated keys for TicketsPage
tickets_detail_title = Ticket Details
tickets_close_btn = Close Ticket
back_to_list = Back to List
tickets_table_category = Category
tickets_priority_label = Priority
tickets_manage_user_btn = Manage User
tickets_msg_user = (User)
tickets_msg_admin = (Admin)
tickets_admin_actions_title = Admin Actions
tickets_update_status_label = Update Status
tickets_status_open = open
tickets_status_in_progress = in progress
tickets_status_resolved = resolved
tickets_status_closed = closed
tickets_reply_label = Reply Message
tickets_reply_placeholder = Enter your reply...
tickets_view_detail_btn = View Detail

plans_form_max_inv_hint = Max Inventory (leave empty for unlimited)
plans_sold_max_label = Sold/Max
plans_unlimited = unlimited
plans_unlimited_traffic = Unlimited Traffic
plans_traffic_value = { $value }GB Traffic
nat_leases_occupied_warning = Occupied by worker, cannot delete
nat_leases_node_desc = Manage public port ranges used by worker for allocation. Each lease corresponds to a public IP port range on a node.
nat_leases_no_nodes_warning = No nodes available. Please add nodes first.
