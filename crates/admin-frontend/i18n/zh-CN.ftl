nav_overview = 系统概览
nav_guests = 用户管理
nav_instances = 实例管理
nav_plans = 产品套餐
nav_nodes = 节点管理
nav_nat_leases = NAT 端口池
nav_tickets = 工单中心
nav_logout = 退出登录
switch_lang = English

# Layout & General
app_title = Cloud Store
admin_console = Admin Console
dash_layout_not_logged_in = 未登录
dash_layout_please_login = 请先登录管理员账号以访问此页面。
dash_layout_go_to_login = 去登录
dash_layout_logout_notice = 已退出管理端会话
dash_layout_admin_desc_p1 = 独立管理端与客户端保持同一视觉系统，但权限和端口隔离。
store_btn = 商店
loading = 加载中...
processing = 处理中...
submit = 提交
save = 保存
cancel = 取消
edit = 编辑
delete = 删除
refresh = 刷新列表
status_label = 状态
actions_label = 操作

# Login Page
login_admin_title = 管理员登录
login_api_base_label = Admin API Base
login_email_label = 电子邮箱
login_password_label = 密码
login_submit_btn = 登录并验证管理员权限
login_err_not_admin = 当前账号不是管理员，无法进入管理端
login_success_notice = 已登录管理员账号
login_err_prefix_profile = 获取个人信息失败: { $err }
login_err_prefix_login = 登录失败: { $err }

# Overview Page
overview_title = 管理面板
overview_desc = 这里管理节点库存、产品上下架、Guest 配置和工单状态。界面壳子与客户中心保持一致，只是内容和权限不同。
overview_current_admin = 当前管理员

# Node Page
nodes_title = 节点管理
nodes_add_btn = 添加节点
nodes_add_modal_title = 添加新节点
nodes_edit_modal_title = 编辑节点: { $name }
nodes_form_name = 节点名称
nodes_form_region = 地区
nodes_form_cpu = CPU 核心
nodes_form_ram = 内存 (MB)
nodes_form_storage = 存储 (GB)
nodes_form_api_endpoint = API 端点 (可选)
nodes_form_api_endpoint_edit = API 端点
nodes_form_incus_token = Incus 信任令牌 (可选)
nodes_form_incus_token_edit = API 令牌
nodes_incus_token_placeholder = token from `incus config trust add <client-name>`
nodes_no_data = 暂无节点数据。
nodes_refresh_success = 节点列表已刷新
nodes_add_success = 节点添加成功
nodes_update_success = 节点更新成功
nodes_error_refresh = 刷新失败: { $err }
nodes_error_add = 添加失败: { $err }
nodes_error_update = 更新失败: { $err }

# Plan Page
plans_title = 产品套餐管理
plans_add_btn = 创建新套餐
plans_no_data = 暂无套餐数据。
plans_form_id = ID (slug)
plans_form_name = 名称
plans_form_desc = 描述
plans_form_price = 价格 (USD/月)
plans_form_cpu = CPU 核心
plans_form_cpu_allowance = CPU 权重 (%)
plans_form_ram = 内存 (MB)
plans_form_storage = 存储 (GB)
plans_form_bw = 端口速度 (Mbps)
plans_form_traffic = 流量限制 (GB, 0为无限)
plans_form_active = 是否激活
plans_add_modal_title = 创建新套餐
plans_edit_modal_title = 编辑套餐: { $name }
plans_add_success = 套餐创建成功
plans_update_success = 套餐更新成功
plans_refresh_success = 套餐列表已刷新

# Instance Page
instances_title = 实例管理
instances_search_placeholder = 搜索实例 ID, 用户 ID...
instances_no_data = 未发现任何实例。
instances_table_id = ID
instances_table_node = 节点
instances_table_user = 用户
instances_table_plan = 套餐
instances_table_status = 状态
instances_table_created = 创建时间
instances_table_image = 操作系统镜像
instances_action_rebuild = 重装系统
instances_action_password = 重置密码
instances_rebuild_modal_title = 重装实例系统
instances_rebuild_confirm = 确定要重装实例 { $id } 吗？所有数据将丢失。
instances_password_modal_title = 重置 Root 密码
instances_password_confirm = 确定要重置实例 { $id } 的 Root 密码吗？
instances_rebuild_success = 重装请求已提交
instances_password_success = 密码重置请求已提交
instances_action_success = 操作已执行

# NAT Leases Page
nat_leases_title = NAT 端口池管理
nat_leases_add_btn = 批量生成端口池
nat_leases_no_data = 暂无端口池数据。
nat_leases_form_node = 选择节点
nat_leases_form_public_ip = 公共 IP (NAT)
nat_leases_form_start_port = 起始端口
nat_leases_form_end_port = 结束端口
nat_leases_generate_success = 端口池生成成功
nat_leases_table_ip = 公共 IP
nat_leases_table_port = 端口
nat_leases_table_target = 目标 (实例:端口)
nat_leases_table_status = 状态
nat_leases_status_available = 可用
nat_leases_status_occupied = 已占用

# Guest Page
guests_title = 用户管理
guests_search_placeholder = 搜索邮箱, 用户 ID...
guests_no_data = 暂无用户数据。
guests_table_id = ID
guests_table_email = 邮箱
guests_table_role = 角色
guests_table_balance = 余额
guests_table_created = 注册时间
guests_action_recharge = 充值余额
guests_recharge_modal_title = 给用户充值: { $email }
guests_form_amount = 充值金额 (USD)
guests_recharge_success = 充值成功

# Ticket Page
tickets_title = 工单中心
tickets_no_data = 暂无工单。
tickets_table_id = ID
tickets_table_user = 用户
tickets_table_subject = 主题
tickets_table_status = 状态
tickets_table_priority = 优先级
tickets_action_detail = 详情
tickets_detail_modal_title = 工单详情: { $subject }
tickets_detail_info = 基本信息
tickets_detail_user = 用户 ID: { $id }
tickets_detail_category = 分类: { $cat }
tickets_detail_priority = 优先级: { $prio }
tickets_detail_status = 状态: { $status }
tickets_detail_msg_admin = (管理员)
tickets_detail_msg_user = (用户)
tickets_detail_reply_placeholder = 输入回复内容...
tickets_detail_send_btn = 发送回复
tickets_detail_close_btn = 关闭工单
tickets_reply_success = 回复成功
tickets_close_success = 工单已关闭

# Dedicated keys for TicketsPage
tickets_detail_title = 工单详情
tickets_close_btn = 关闭工单
back_to_list = 返回列表
tickets_table_category = 分类
tickets_priority_label = 优先级
tickets_manage_user_btn = 管理用户
tickets_msg_user = (用户)
tickets_msg_admin = (管理员)
tickets_admin_actions_title = 管理操作
tickets_update_status_label = 修改状态
tickets_status_open = 开启
tickets_status_in_progress = 处理中
tickets_status_resolved = 已解决
tickets_status_closed = 已关闭
tickets_reply_label = 回复内容
tickets_reply_placeholder = 输入回复内容...
tickets_view_detail_btn = 查看详情

plans_form_max_inv_hint = 库存上限（留空表示不限）
plans_sold_max_label = 已售/上限
plans_unlimited = 不限
plans_unlimited_traffic = 无限流量
plans_traffic_value = { $value }GB 流量
nat_leases_occupied_warning = 已被 worker 占用，不能删除
nat_leases_node_desc = 这里管理 worker 用来给实例分配的端口段。每条租约对应一个节点上的一段公网端口范围。
nat_leases_no_nodes_warning = 当前还没有可选节点，请先到 Nodes 页面添加节点。

