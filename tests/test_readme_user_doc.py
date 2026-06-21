import unittest
from pathlib import Path


class ReadmeUserDocTests(unittest.TestCase):
    def test_readme_is_official_user_facing_documentation(self) -> None:
        readme = Path("README.md").read_text(encoding="utf-8")

        self.assertTrue(readme.startswith("# 剪贴板同步"))
        for section in (
            "## 产品简介",
            "## 主要功能",
            "## 适用环境",
            "## 快速开始",
            "## 连接另一台设备",
            "## 修改设备 ID 和端口",
            "## 使用注意事项",
        ):
            self.assertIn(section, readme)

        for developer_copy in (
            "MVP",
            "clipboard-sync-design.md",
            "UI UX Pro Max",
            "localhost:61237",
            "python -m unittest",
            "通信协议",
        ):
            self.assertNotIn(developer_copy, readme)


if __name__ == "__main__":
    unittest.main()
