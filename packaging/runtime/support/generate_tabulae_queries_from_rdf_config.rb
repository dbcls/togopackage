#!/usr/bin/env ruby
# frozen_string_literal: true

require 'yaml'
require 'date'
require 'fileutils'
require 'optparse'
require 'set'

require_relative 'rdf_config_query_generator'

def parse_options
  opts = {
    rdf_config_base_dir: '/data/rdf-config',
    output_dir: nil,
    sparql_endpoint: 'http://localhost:7002/sparql',
    paginate: 10_000
  }

  OptionParser.new do |parser|
    parser.on('--rdf-config-base-dir DIR') { |v| opts[:rdf_config_base_dir] = v }
    parser.on('--output-dir DIR') { |v| opts[:output_dir] = v }
    parser.on('--sparql-endpoint URL') { |v| opts[:sparql_endpoint] = v }
    parser.on('--paginate N', Integer) { |v| opts[:paginate] = v }
  end.parse!

  if opts[:output_dir].to_s.strip.empty?
    warn 'Usage: generate_tabulae_queries_from_rdf_config.rb --output-dir DIR [--rdf-config-base-dir DIR]'
    exit(1)
  end

  opts
end

def load_yaml(path)
  YAML.load_file(path, permitted_classes: [Date, Time, Symbol])
rescue StandardError => e
  raise "Failed to load YAML: #{path} (#{e.message})"
end

def sanitize_filename(name)
  value = name.to_s.downcase.gsub(/[^a-z0-9_-]+/, '_').gsub(/\A_+|_+\z/, '')
  value.empty? ? 'query' : value
end

def pick_endpoint(override_endpoint)
  endpoint = override_endpoint.to_s.strip
  raise 'SPARQL endpoint is empty' if endpoint.empty?

  endpoint
end

def pick_paginate(value)
  paginate = value.to_i
  raise 'Paginate value must be greater than 0' unless paginate.positive?

  paginate
end

def generate_sparql(config_dir, query_name)
  RDFConfigQueryGenerator.generate_named_query(config_dir, query_name)
end

def generate_model_query(config_dir, subject_name, attribute_name)
  RDFConfigQueryGenerator.generate_model_query(config_dir, subject_name, attribute_name)
end

def normalize_key(key)
  value = key.to_s.strip
  value = value.sub(/\{[^}]+\}\z/, '')
  value.sub(/[+*?]\z/, '')
end

def load_prefixes(config_dir)
  path = File.join(config_dir, 'prefix.yaml')
  return {} unless File.exist?(path)

  prefixes = load_yaml(path)
  return {} unless prefixes.is_a?(Hash)

  prefixes.transform_keys(&:to_s)
end

def expand_term(term, prefixes)
  raw = term.to_s.strip
  return nil if raw.empty?
  return nil if raw.start_with?('"', "'")
  return nil if raw == '[]'

  if raw.start_with?('<') && raw.end_with?('>')
    return raw[1...-1]
  end
  return raw if raw.start_with?('http://', 'https://')

  prefix, local = raw.split(':', 2)
  return nil if local.nil?

  namespace = prefixes["#{prefix}:"] || prefixes[prefix]
  return nil unless namespace

  namespace.to_s.delete_prefix('<').delete_suffix('>') + local
end

def attribute_name?(key)
  return false if key.empty?
  return false if key == 'a' || key == '[]'
  return false if key.include?(':')

  true
end

def collect_attribute_paths(node, current_path, prefixes, attribute_paths)
  case node
  when Array
    node.each { |item| collect_attribute_paths(item, current_path, prefixes, attribute_paths) }
  when Hash
    node.each do |key, value|
      normalized = normalize_key(key)
      if attribute_name?(normalized)
        path_expression = current_path.map { |iri| "<#{iri}>" }.join('/')
        if !path_expression.empty? && !attribute_paths[normalized].include?(path_expression)
          attribute_paths[normalized] << path_expression
        end
        next
      end

      next if normalized == 'a'

      if normalized == '[]'
        collect_attribute_paths(value, current_path, prefixes, attribute_paths)
        next
      end

      predicate_iri = expand_term(normalized, prefixes)
      next unless predicate_iri

      collect_attribute_paths(value, current_path + [predicate_iri], prefixes, attribute_paths)
    end
  end
end

def extract_class_attribute_specs(model_config, prefixes)
  class_attribute_specs = {}
  return class_attribute_specs unless model_config.is_a?(Array)

  model_config.each do |entry|
    next unless entry.is_a?(Hash)

    entry.each do |subject_key, attributes|
      next unless attributes.is_a?(Array)

      subject_name = subject_key.to_s.split(/\s+/, 2).first
      next if subject_name.to_s.strip.empty?

      class_iris = []
      attributes.each do |attribute_block|
        next unless attribute_block.is_a?(Hash)

        attribute_block.each do |predicate, object|
          next unless normalize_key(predicate) == 'a'

          values = object.is_a?(Array) ? object : object.to_s.split(/\s*,\s*/)
          values.each do |value|
            iri = expand_term(value, prefixes)
            class_iris << iri if iri
          end
        end
      end

      class_iri = class_iris.uniq.sort.first
      next unless class_iri

      attribute_paths = Hash.new { |h, k| h[k] = [] }
      collect_attribute_paths(attributes, [], prefixes, attribute_paths)
      next if attribute_paths.empty?

      class_attribute_specs[subject_name] = {
        class_iri: class_iri,
        attributes: attribute_paths
      }
    end
  end

  class_attribute_specs
end

def ensure_tabulae_endpoint(query, endpoint)
  lines = query.lines.map(&:rstrip)
  filtered = lines.reject { |line| line.match?(/^#\s*(Endpoint|Paginate)\s*:/i) }
  (["# Endpoint: #{endpoint}"] + filtered).join("\n") + "\n"
end

def ensure_tabulae_paginate(query, paginate)
  lines = query.lines.map(&:rstrip)
  filtered = lines.reject { |line| line.match?(/^#\s*Paginate\s*:/i) }
  (["# Paginate: #{paginate}"] + filtered).join("\n") + "\n"
end

def remove_trailing_limit(query)
  lines = query.lines.map(&:rstrip)
  while !lines.empty? && lines.last.empty?
    lines.pop
  end
  if !lines.empty? && lines.last.match?(/^LIMIT\s+\d+\s*$/i)
    lines.pop
  end
  lines.join("\n") + "\n"
end

def write_queries(opts)
  base_dir = File.expand_path(opts[:rdf_config_base_dir])
  output_dir = File.expand_path(opts[:output_dir])
  endpoint = pick_endpoint(opts[:sparql_endpoint])
  paginate = pick_paginate(opts[:paginate])

  raise "RDF-config base directory does not exist: #{base_dir}" unless Dir.exist?(base_dir)

  FileUtils.mkdir_p(output_dir)
  count = 0
  used_names = {}

  Dir.children(base_dir).sort.each do |entry|
    config_dir = File.join(base_dir, entry)
    next unless File.directory?(config_dir)
    next unless File.exist?(File.join(config_dir, 'model.yaml'))
    next unless File.exist?(File.join(config_dir, 'sparql.yaml'))

    sparql_config = load_yaml(File.join(config_dir, 'sparql.yaml'))
    raise "Invalid sparql.yaml format: #{config_dir}" unless sparql_config.is_a?(Hash)

    model_config = load_yaml(File.join(config_dir, 'model.yaml'))
    prefixes = load_prefixes(config_dir)
    class_attribute_specs = extract_class_attribute_specs(model_config, prefixes)
    class_attribute_query_names = class_attribute_specs.flat_map do |subject_name, spec|
      subject = sanitize_filename(subject_name)
      spec[:attributes].keys.map { |attribute_name| "#{subject}__#{sanitize_filename(attribute_name)}" }
    end.to_set

    sparql_config.each_key do |query_name|
      next if query_name.to_s.strip.empty?
      next if query_name.to_s == 'databaseinfo__all_attributes'
      next if class_attribute_query_names.include?(sanitize_filename(query_name))

      generated_query = generate_sparql(config_dir, query_name.to_s)
      query = ensure_tabulae_endpoint(generated_query, endpoint)
      query = ensure_tabulae_paginate(query, paginate)
      query = remove_trailing_limit(query)

      filename_base = "#{sanitize_filename(entry)}__#{sanitize_filename(query_name)}"
      filename = "#{filename_base}.rq"
      if used_names.key?(filename)
        used_names[filename] += 1
        filename = "#{filename_base}_#{used_names[filename]}.rq"
      else
        used_names[filename] = 0
      end

      File.write(File.join(output_dir, filename), query)
      count += 1
    end

    class_attribute_specs.each do |subject_name, spec|
      spec[:attributes].keys.sort.each do |attribute_name|
        query = generate_model_query(config_dir, subject_name, attribute_name)
        query = ensure_tabulae_endpoint(query, endpoint)
        query = ensure_tabulae_paginate(query, paginate)
        query = remove_trailing_limit(query)
        filename_base = [
          sanitize_filename(entry),
          sanitize_filename(subject_name),
          sanitize_filename(attribute_name)
        ].join('__')
        filename = "#{filename_base}.rq"
        if used_names.key?(filename)
          used_names[filename] += 1
          filename = "#{filename_base}_#{used_names[filename]}.rq"
        else
          used_names[filename] = 0
        end

        File.write(File.join(output_dir, filename), query)
        count += 1
      end
    end
  end

  if count.zero?
    warn 'No Tabulae queries generated from RDF-config.'
    exit 10
  end

  count
end

options = parse_options
count = write_queries(options)
puts "Generated #{count} Tabulae query files."
