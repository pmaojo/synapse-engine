import frappe

def execute():
    """
    Seed default plans for Grafoso SaaS.
    Run via: bench execute scripts.seed_plans.execute
    """
    plans = [
        {
            "plan_name": "Starter",
            "price": 0,
            "max_projects": 1,
            "max_storage_mb": 100,
            "max_training_hours": 1
        },
        {
            "plan_name": "Pro",
            "price": 49,
            "max_projects": 10,
            "max_storage_mb": 10000,
            "max_training_hours": 20
        },
        {
            "plan_name": "Enterprise",
            "price": 299,
            "max_projects": 100,
            "max_storage_mb": 100000,
            "max_training_hours": 100
        }
    ]

    for p in plans:
        if not frappe.db.exists("Plan", {"plan_name": p["plan_name"]}):
            plan_doc = frappe.get_doc({
                "doctype": "Plan",
                **p
            })
            plan_doc.insert(ignore_permissions=True)
            print(f"Created plan: {p['plan_name']}")
        else:
            print(f"Plan exists: {p['plan_name']}")

    frappe.db.commit()
