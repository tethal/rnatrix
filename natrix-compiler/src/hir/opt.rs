use crate::error::{err_at, AttachErrSpan, SourceResult};
use crate::hir::{Expr, ExprKind, GlobalKind, Program, Stmt, StmtKind};
use natrix_runtime::value::{Value, ValueType};

pub fn fold_constants(program: &mut Program) -> SourceResult<()> {
    for global in program.globals.iter_mut() {
        match &mut global.kind {
            GlobalKind::Function(fun_decl) => do_block(&mut fun_decl.body)?,
        }
    }
    Ok(())
}

fn do_block(stmts: &mut Vec<Stmt>) -> SourceResult<()> {
    for stmt in stmts.iter_mut() {
        do_stmt(stmt)?;
    }
    Ok(())
}

fn do_stmt(stmt: &mut Stmt) -> SourceResult<()> {
    match &mut stmt.kind {
        StmtKind::Block(stmts) => do_block(stmts),
        StmtKind::Break(_) => Ok(()),
        StmtKind::Continue(_) => Ok(()),
        StmtKind::Expr(expr) => {
            do_expr(expr)?;
            Ok(())
        }
        StmtKind::If(cond, then_body, else_body) => {
            do_bool_expr(cond)?;
            do_stmt(then_body)?;
            if let Some(else_body) = else_body {
                do_stmt(else_body)?;
            }
            Ok(())
        }
        StmtKind::Return(expr) => {
            do_expr(expr)?;
            Ok(())
        }
        StmtKind::SetItem(array, index, value) => {
            do_expr(array)?;
            do_expr(index)?;
            do_expr(value)?;
            Ok(())
        }
        StmtKind::StoreGlobal(_, expr) => {
            do_expr(expr)?;
            Ok(())
        }
        StmtKind::StoreLocal(_, expr) => {
            do_expr(expr)?;
            Ok(())
        }
        StmtKind::VarDecl(_, expr) => {
            do_expr(expr)?;
            Ok(())
        }
        StmtKind::While(_, cond, body) => {
            do_bool_expr(cond)?;
            do_stmt(body)
        }
    }
}

fn do_expr(expr: &mut Expr) -> SourceResult<Option<Value>> {
    let value = match &mut expr.kind {
        ExprKind::Binary(op, op_span, left, right) => {
            if let (Some(left), Some(right)) = (do_expr(left)?, do_expr(right)?) {
                Some(op.eval(&left, &right).err_at(*op_span)?)
            } else {
                None
            }
        }
        ExprKind::Call(callee, args) => {
            do_expr(callee)?;
            let values: Vec<Option<Value>> = args
                .iter_mut()
                .map(|arg| do_expr(arg))
                .collect::<Result<_, _>>()?;
            if let ExprKind::LoadBuiltin(builtin) = callee.kind
                && let Some(values) = values.into_iter().collect::<Option<Vec<_>>>()
            {
                builtin.eval_const(&values).err_at(expr.span)?
            } else {
                None
            }
        }
        ExprKind::ConstBool(v) => Some(Value::from_bool(*v)),
        ExprKind::ConstFloat(v) => Some(Value::from_float(*v)),
        ExprKind::ConstInt(v) => Some(Value::from_int(*v)),
        ExprKind::ConstNull => Some(Value::NULL),
        ExprKind::ConstString(v) => Some(Value::from_string(v.clone())),
        ExprKind::GetItem(array, index) => {
            if let (Some(array), Some(index)) = (do_expr(array)?, do_expr(index)?) {
                Some(array.get_item(index).err_at(expr.span)?)
            } else {
                // Possible future optimization (not constant folding): if array is a list literal
                // and index is constant, could evaluate all elements for side effects but extract
                // only the indexed one. Complex and low-value, so deferred.
                None
            }
        }
        ExprKind::LoadBuiltin(_) => None,
        ExprKind::LoadGlobal(_) => None,
        ExprKind::LoadLocal(_) => None,
        ExprKind::LogicalBinary(and, _, left, right) => {
            if let Some(left) = do_bool_expr(left)? {
                if (*and && !left) || (!*and && left) {
                    // lhs determines result, no need to evaluate rhs (short-circuit)
                    Some(Value::from_bool(left))
                } else {
                    do_bool_expr(right)?.map(Value::from_bool)
                }
            } else {
                // do not fold - lhs might have side effects
                do_expr(right)?;
                None
            }
        }
        ExprKind::MakeList(exprs) => {
            for expr in exprs.iter_mut() {
                do_expr(expr)?;
            }
            None
        }
        ExprKind::Unary(op, op_span, expr) => {
            if let Some(expr) = do_expr(expr)? {
                Some(op.eval(&expr).err_at(*op_span)?)
            } else {
                None
            }
        }
    };

    // If we got a value, replace the expression
    if let Some(val) = &value {
        expr.kind = match val.get_type() {
            ValueType::Null => ExprKind::ConstNull,
            ValueType::Bool => ExprKind::ConstBool(val.unwrap_bool()),
            ValueType::Int => ExprKind::ConstInt(val.unwrap_int()),
            ValueType::Float => ExprKind::ConstFloat(val.unwrap_float()),
            ValueType::String => ExprKind::ConstString(val.unwrap_string()),
            ValueType::List | ValueType::Function => unreachable!(),
        };
    }

    Ok(value)
}

fn do_bool_expr(expr: &mut Expr) -> SourceResult<Option<bool>> {
    if let Some(value) = do_expr(expr)? {
        if value.is_bool() {
            Ok(Some(value.unwrap_bool()))
        } else {
            err_at(expr.span, "expected a boolean value")
        }
    } else {
        Ok(None)
    }
}
