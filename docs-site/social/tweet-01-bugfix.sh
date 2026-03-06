#!/bin/bash
# ReX CLI 推文配图生成脚本

# 设置颜色
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
GRAY='\033[0;90m'
NC='\033[0m' # No Color

# 背景色
BG_GREEN='\033[42m'
BG_BLUE='\033[44m'

echo ""
echo -e "${BG_BLUE}${WHITE}  ReX CLI - 让 AI 帮你修 Bug  ${NC}"
echo ""
echo -e "${GRAY}─────────────────────────────────────────────────────────${NC}"
echo ""
echo -e "${GREEN}➜${NC} ${WHITE}rex run --prompt \"修复登录失败的问题\""
echo ""
echo -e "${CYAN}[1/5]${NC} 🔍 分析代码库..."
echo -e "   Found: 3 potential issues in auth/login.py"
echo ""
echo -e "${CYAN}[2/5]${NC} 🧪 运行测试..."
echo -e "   ✗ test_login_expired_token (failed)"
echo ""
echo -e "${CYAN}[3/5]${NC} 🔧 应用修复..."
echo -e "   ✓ Fixed: token expiry calculation at line 47"
echo ""
echo -e "${CYAN}[4/5]${NC} ✅ 验证修复..."
echo -e "   ✓ All tests passed (12/12)"
echo ""
echo -e "${CYAN}[5/5]${NC} 💾 自动保存..."
echo -e "   ✓ Commit: fix(login): resolve token expiry bug"
echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ 完成！耗时 23s${NC}"
echo ""
echo -e "${GRAY}─────────────────────────────────────────────────────────${NC}"
echo ""
