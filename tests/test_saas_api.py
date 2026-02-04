
import sys
import unittest
from unittest.mock import MagicMock, patch

# Mock the frappe module before importing api
sys.modules["frappe"] = MagicMock()
import frappe
from frappe import _

# Setup specific mocks for frappe functions
frappe.whitelist = lambda: lambda x: x
frappe.throw = MagicMock(side_effect=Exception("Frappe Throw"))

# Import the module to test
# We need to add the path to sys.path so we can import it
import os
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "../frappe_app/grafoso_saas/grafoso_saas")))

import api

class TestSaasAPI(unittest.TestCase):

    def setUp(self):
        frappe.db.get_value.reset_mock()
        frappe.get_all.reset_mock()
        frappe.get_doc.reset_mock()
        frappe.db.set_value.reset_mock()
        frappe.db.exists.reset_mock()
        frappe.session.user = "test@example.com"
        # Reset side_effect for get_value
        frappe.db.get_value.side_effect = None

    def test_get_subscription_status_no_tenant(self):
        frappe.db.get_value.return_value = None
        # Mock frappe.throw to raise an exception with the message
        frappe.throw.side_effect = Exception("Tenant not found for user")

        # When get_value returns None, api.py calls frappe.throw
        # We need to ensure that logic is hit.
        # In api.py: if not tenant_name: frappe.throw(...)

        with self.assertRaises(Exception) as cm:
            api.get_subscription_status()
        self.assertIn("Tenant not found", str(cm.exception))

    def test_get_subscription_status_inactive(self):
        frappe.db.get_value.return_value = "TEN-00001"
        frappe.get_all.return_value = []

        result = api.get_subscription_status()
        self.assertEqual(result["status"], "Inactive")

    def test_get_subscription_status_active(self):
        frappe.db.get_value.return_value = "TEN-00001"

        mock_sub = MagicMock()
        mock_sub.name = "SUB-00001"
        mock_sub.plan = "Starter"
        mock_sub.status = "Active"
        mock_sub.start_date = "2024-01-01"
        mock_sub.end_date = "2024-02-01"

        # frappe.get_all returns a list of dictionaries usually, or objects if accessing attributes
        # mimicking behavior where we access attributes
        frappe.get_all.return_value = [mock_sub]

        mock_plan = MagicMock()
        mock_plan.plan_name = "Starter"
        mock_plan.max_projects = 5
        mock_plan.max_storage_mb = 1000
        mock_plan.max_training_hours = 10
        frappe.get_doc.return_value = mock_plan

        result = api.get_subscription_status()

        self.assertEqual(result["status"], "Active")
        self.assertEqual(result["plan"]["name"], "Starter")
        self.assertEqual(result["plan"]["max_projects"], 5)

    def test_change_plan(self):
        # First for tenant, second for Plan name lookup
        # We need to make sure we don't exhaust the side_effect in other tests or here if called differently
        frappe.db.get_value.side_effect = None
        frappe.db.get_value.return_value = "TEN-00001"

        def get_value_side_effect(*args, **kwargs):
            if args[0] == "Tenant": return "TEN-00001"
            if args[0] == "Plan": return "Starter"
            return None

        frappe.db.get_value.side_effect = get_value_side_effect

        frappe.db.exists.return_value = True
        frappe.get_all.return_value = [] # No active subs to cancel

        mock_new_sub = MagicMock()
        mock_new_sub.name = "SUB-00002"
        frappe.get_doc.return_value = mock_new_sub

        result = api.change_plan("Pro")

        self.assertEqual(result["message"], "Plan changed successfully")
        self.assertEqual(result["new_subscription"], "SUB-00002")
        frappe.db.set_value.assert_called_with("Tenant", "TEN-00001", "current_plan", "Pro")

    def test_get_usage_stats(self):
        frappe.db.get_value.return_value = "TEN-00001"

        mock_log1 = MagicMock()
        mock_log1.resource_type = "Storage (MB)"
        mock_log1.amount = 100

        mock_log2 = MagicMock()
        mock_log2.resource_type = "Training (Hours)"
        mock_log2.amount = 2.5

        frappe.get_all.return_value = [mock_log1, mock_log2]

        result = api.get_usage_stats()

        self.assertEqual(result["Storage (MB)"], 100)
        self.assertEqual(result["Training (Hours)"], 2.5)
        self.assertEqual(result["Triple Extraction (Count)"], 0)

if __name__ == '__main__':
    unittest.main()
