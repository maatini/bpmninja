import { Trash } from 'lucide-react';

// Shared type definitions for the variable editor
export type VarType = 'String' | 'Number' | 'Boolean' | 'Object' | 'Null';

export interface VariableRow {
  name: string;
  type: VarType;
  value: unknown;
  isNew?: boolean;
}

/**
 * Parse a variables record (as returned from the engine) into VariableRow[].
 */
export function parseVariables(vars: Record<string, unknown>): VariableRow[] {
  return Object.entries(vars)
    .sort((a, b) => a[0].localeCompare(b[0]))
    .map(([name, val]) => {
      let type: VarType = 'String';
      if (val === null) type = 'Null';
      else if (typeof val === 'boolean') type = 'Boolean';
      else if (typeof val === 'number') type = 'Number';
      else if (typeof val === 'object') type = 'Object';

      return {
        name,
        type,
        value: type === 'Object' ? JSON.stringify(val, null, 2) : val,
      };
    });
}

/**
 * Serialize VariableRow[] back into a plain Record<string, unknown>.
 * Returns null and shows an alert if validation fails.
 */
export function serializeVariables(
  variables: VariableRow[],
  deletedKeys?: Set<string>,
): Record<string, unknown> | null {
  const result: Record<string, unknown> = {};

  // Mark explicitly deleted keys as null
  if (deletedKeys) {
    for (const key of deletedKeys) {
      result[key] = null;
    }
  }

  for (const v of variables) {
    if (!v.name.trim()) continue; // skip unnamed variables

    if (v.type === 'Object') {
      try {
        result[v.name] = JSON.parse(v.value as string);
      } catch {
        alert(`Invalid JSON for variable '${v.name}'`);
        return null;
      }
    } else if (v.type === 'Number') {
      const num = Number(v.value);
      if (isNaN(num)) {
        alert(`Invalid number for variable '${v.name}'`);
        return null;
      }
      result[v.name] = num;
    } else if (v.type === 'Boolean') {
      result[v.name] = Boolean(v.value);
    } else if (v.type === 'Null') {
      result[v.name] = null;
    } else {
      result[v.name] = v.value;
    }
  }

  return result;
}

interface VariableEditorProps {
  variables: VariableRow[];
  onChange: (variables: VariableRow[]) => void;
  /** If true, existing variable names are read-only (used in Instance detail view). */
  readOnlyNames?: boolean;
  /** Track deleted keys for backend synchronisation (optional). */
  deletedKeys?: Set<string>;
  onDeletedKeysChange?: (keys: Set<string>) => void;
}

/**
 * Reusable typed variable editor table with Name / Type / Value columns
 * and Add / Remove controls.
 */
export function VariableEditor({
  variables,
  onChange,
  readOnlyNames = false,
  deletedKeys,
  onDeletedKeysChange,
}: VariableEditorProps) {
  const handleChange = (index: number, field: keyof VariableRow, newValue: unknown) => {
    const updated = [...variables];
    const row = { ...updated[index] };

    if (field === 'type') {
      row.type = newValue as VarType;
      if (row.type === 'String') row.value = '';
      else if (row.type === 'Number') row.value = 0;
      else if (row.type === 'Boolean') row.value = false;
      else if (row.type === 'Null') row.value = null;
      else if (row.type === 'Object') row.value = '{}';
    } else {
      (row as Record<string, unknown>)[field] = newValue;
    }

    updated[index] = row;
    onChange(updated);
  };

  const handleAdd = () => {
    onChange([...variables, { name: '', type: 'String', value: '', isNew: true }]);
  };

  const handleRemove = (index: number) => {
    const updated = [...variables];
    const removed = updated.splice(index, 1)[0];

    // Track removed keys that existed on the backend
    if (!removed.isNew && removed.name.trim() && deletedKeys && onDeletedKeysChange) {
      const newDeleted = new Set(deletedKeys);
      newDeleted.add(removed.name);
      onDeletedKeysChange(newDeleted);
    }

    onChange(updated);
  };

  return (
    <>
      <table className="variables-table">
        <thead>
          <tr>
            <th style={{ width: '25%' }}>Name</th>
            <th style={{ width: '20%' }}>Type</th>
            <th>Value</th>
            <th style={{ width: '40px', textAlign: 'center' }}></th>
          </tr>
        </thead>
        <tbody>
          {variables.map((v, idx) => (
            <tr key={idx}>
              <td>
                <input
                  type="text"
                  className="var-input"
                  value={v.name}
                  onChange={(e) => handleChange(idx, 'name', e.target.value)}
                  placeholder="Variable name"
                  readOnly={readOnlyNames && !v.isNew}
                  style={{ backgroundColor: readOnlyNames && !v.isNew ? '#f8fafc' : '#ffffff' }}
                />
              </td>
              <td>
                <select
                  className="var-select"
                  value={v.type}
                  onChange={(e) => handleChange(idx, 'type', e.target.value)}
                >
                  <option value="String">String</option>
                  <option value="Number">Number</option>
                  <option value="Boolean">Boolean</option>
                  <option value="Object">Object</option>
                  <option value="Null">Null</option>
                </select>
              </td>
              <td>
                {v.type === 'String' && (
                  <input
                    type="text"
                    className="var-input"
                    value={v.value as string}
                    onChange={(e) => handleChange(idx, 'value', e.target.value)}
                    placeholder="String value"
                  />
                )}
                {v.type === 'Number' && (
                  <input
                    type="number"
                    className="var-input"
                    value={v.value as number}
                    onChange={(e) => handleChange(idx, 'value', parseFloat(e.target.value))}
                    placeholder="Number value"
                  />
                )}
                {v.type === 'Boolean' && (
                  <input
                    type="checkbox"
                    className="var-checkbox"
                    checked={v.value as boolean}
                    onChange={(e) => handleChange(idx, 'value', e.target.checked)}
                  />
                )}
                {v.type === 'Object' && (
                  <textarea
                    className="vars-textarea"
                    value={v.value as string}
                    onChange={(e) => handleChange(idx, 'value', e.target.value)}
                    rows={2}
                    spellCheck={false}
                    style={{ width: '100%', resize: 'vertical' }}
                  />
                )}
                {v.type === 'Null' && (
                  <span style={{ color: '#94a3b8', fontStyle: 'italic', fontSize: '0.85rem' }}>null</span>
                )}
              </td>
              <td style={{ textAlign: 'center' }}>
                <button
                  className="button"
                  style={{ background: 'transparent', color: '#ef4444', border: 'none', padding: '4px', cursor: 'pointer' }}
                  onClick={() => handleRemove(idx)}
                  title="Delete Variable"
                >
                  <Trash size={16} />
                </button>
              </td>
            </tr>
          ))}
          {variables.length === 0 && (
            <tr>
              <td colSpan={4} style={{ textAlign: 'center', color: '#64748b', padding: '16px' }}>
                No variables configured.
              </td>
            </tr>
          )}
        </tbody>
      </table>
      <div style={{ marginTop: 12, display: 'flex', justifyContent: 'flex-start' }}>
        <button
          className="button"
          onClick={handleAdd}
          style={{ background: '#f1f5f9', color: '#334155', border: '1px solid #cbd5e1' }}
        >
          + Add Variable
        </button>
      </div>
    </>
  );
}
